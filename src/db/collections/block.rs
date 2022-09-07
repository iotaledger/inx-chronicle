// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::doc,
    error::Error,
    options::{IndexOptions, InsertManyOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::PayloadKind;
use crate::{
    db::{
        collections::OutputCollection,
        mongodb::{InsertIgnoreDuplicatesExt, MongoCollectionExt, MongoDbCollection},
        MongoDb,
    },
    types::{
        ledger::{BlockMetadata, LedgerInclusionState},
        stardust::block::{output::OutputId, payload::transaction::TransactionId, Block, BlockId},
        tangle::MilestoneIndex,
    },
};

/// Chronicle Block record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockDocument {
    #[serde(rename = "_id")]
    block_id: BlockId,
    /// The block.
    block: Block,
    /// The raw bytes of the block.
    #[serde(with = "serde_bytes")]
    raw: Vec<u8>,
    /// The block's metadata.
    metadata: BlockMetadata,
}

/// The stardust blocks collection.
pub struct BlockCollection {
    collection: mongodb::Collection<BlockDocument>,
}

impl MongoDbCollection for BlockCollection {
    const NAME: &'static str = "stardust_blocks";
    type Document = BlockDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

/// Implements the queries for the core API.
impl BlockCollection {
    /// Creates block indexes.
    pub async fn create_indexes(&self) -> Result<(), Error> {
        self.create_index(
            IndexModel::builder()
                .keys(doc! { "block.payload.transaction_id": 1 })
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .name("transaction_id_index".to_string())
                        .partial_filter_expression(doc! {
                            "block.payload.transaction_id": { "$exists": true },
                            "metadata.inclusion_state": { "$eq": LedgerInclusionState::Included },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }

    /// Get a [`Block`] by its [`BlockId`].
    pub async fn get_block(&self, block_id: &BlockId) -> Result<Option<Block>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": { "_id": block_id } },
                doc! { "$lookup": {
                    "from": OutputCollection::NAME,
                    "localField": "_id",
                    "foreignField": "metadata.block_id",
                    "pipeline": [
                        { "$replaceWith": "$output" }
                    ],
                    "as": "block.payload.essence.outputs"
                } },
                doc! { "$replaceWith": "$block" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Get the raw bytes of a [`Block`] by its [`BlockId`].
    pub async fn get_block_raw(&self, block_id: &BlockId) -> Result<Option<Vec<u8>>, Error> {
        #[derive(Deserialize)]
        struct RawResult {
            #[serde(with = "serde_bytes")]
            data: Vec<u8>,
        }

        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": { "_id": block_id } },
                    doc! { "$replaceWith": { "data": "$raw" } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|RawResult { data }| data))
    }

    /// Get the metadata of a [`Block`] by its [`BlockId`].
    pub async fn get_block_metadata(&self, block_id: &BlockId) -> Result<Option<BlockMetadata>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": { "_id": block_id } },
                doc! { "$replaceWith": "$metadata" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Get the children of a [`Block`] as a stream of [`BlockId`]s.
    pub async fn get_block_children(
        &self,
        block_id: &BlockId,
        page_size: usize,
        page: usize,
    ) -> Result<impl Stream<Item = Result<BlockId, Error>>, Error> {
        #[derive(Deserialize)]
        struct BlockIdResult {
            block_id: BlockId,
        }

        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": { "block.parents": block_id } },
                    doc! { "$skip": (page_size * page) as i64 },
                    doc! { "$sort": {"metadata.referenced_by_milestone_index": -1} },
                    doc! { "$limit": page_size as i64 },
                    doc! { "$replaceWith": { "block_id": "$_id" } },
                ],
                None,
            )
            .await?
            .map_ok(|BlockIdResult { block_id }| block_id))
    }

    /// Inserts [`Block`]s together with their associated [`BlockMetadata`].
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_blocks_with_metadata<I>(&self, blocks_with_metadata: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = (BlockId, Block, Vec<u8>, BlockMetadata)>,
        I::IntoIter: Send + Sync,
    {
        let blocks_with_metadata = blocks_with_metadata
            .into_iter()
            .map(|(block_id, block, raw, metadata)| BlockDocument {
                block_id,
                block,
                raw,
                metadata,
            });

        self.insert_many_ignore_duplicates(
            blocks_with_metadata,
            InsertManyOptions::builder().ordered(false).build(),
        )
        .await?;

        Ok(())
    }

    /// Finds the [`Block`] that included a transaction by [`TransactionId`].
    pub async fn get_block_for_transaction(&self, transaction_id: &TransactionId) -> Result<Option<Block>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": {
                    "metadata.inclusion_state": LedgerInclusionState::Included,
                    "block.payload.transaction_id": transaction_id,
                } },
                doc! { "$lookup": {
                    "from": OutputCollection::NAME,
                    "localField": "_id",
                    "foreignField": "metadata.block_id",
                    "pipeline": [
                        { "$replaceWith": "$output" }
                    ],
                    "as": "block.payload.essence.outputs"
                } },
                doc! { "$replaceWith": "$block" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Gets the spending transaction of an [`Output`](crate::types::stardust::block::Output) by [`OutputId`].
    pub async fn get_spending_transaction(&self, output_id: &OutputId) -> Result<Option<Block>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": {
                    "metadata.inclusion_state": LedgerInclusionState::Included,
                    "block.payload.essence.inputs.transaction_id": &output_id.transaction_id,
                    "block.payload.essence.inputs.index": &(output_id.index as i32)
                } },
                doc! { "$lookup": {
                    "from": OutputCollection::NAME,
                    "localField": "_id",
                    "foreignField": "metadata.block_id",
                    "pipeline": [
                        { "$replaceWith": "$output" }
                    ],
                    "as": "block.payload.essence.outputs"
                } },
                doc! { "$replaceWith": "$block" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct BlockAnalyticsResult {
    pub count: u64,
}

impl BlockCollection {
    /// Gathers block analytics.
    pub async fn get_block_analytics<B: PayloadKind>(
        &self,
        start_index: Option<MilestoneIndex>,
        end_index: Option<MilestoneIndex>,
    ) -> Result<BlockAnalyticsResult, Error> {
        let mut queries = vec![doc! {
            "$nor": [
                { "metadata.referenced_by_milestone_index": { "$lt": start_index } },
                { "metadata.referenced_by_milestone_index": { "$gte": end_index } },
            ],
        }];
        if let Some(kind) = B::kind() {
            queries.push(doc! { "block.payload.kind": kind });
        }
        Ok(self
            .aggregate(
                vec![doc! { "$match": { "$and": queries } }, doc! { "$count": "count" }],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
    }
}

/// The milestone's details (number of referenced blocks, ledger mutations etc.).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MilestoneAnalyticsResult {
    /// The number of blocks referenced by a milestone.
    pub num_blocks: u32,
    /// The number of blocks referenced by a milestone that contain no payload.
    pub num_no_payload: u32,
    // /// The number of blocks referenced by a milestone that contain a payload.
    // pub num_tx_payload: u32,
    // /// The number of blocks containing a treasury transaction payload.
    // pub num_treasury_tx_payload: u32,
    // /// The number of blocks containing a tagged data payload.
    // pub num_tagged_data_payload: u32,
    // /// The number of blocks containing a milestone payload.
    // pub num_milestone_payload: u32,
    // /// The number of blocks containing a confirmed transaction.
    // pub num_confirmed: u32,
    // /// The number of blocks containing a conflicting transaction.
    // pub num_conflicting: u32,
}

impl BlockCollection {
    /// Gets the [`MilestoneStats`] of a milestone.
    pub async fn get_milestone_analytics(&self, index: &MilestoneIndex) -> Result<MilestoneAnalyticsResult, Error> {
        Ok(self.aggregate(
            vec![
                doc! { "$match": { "metadata.referenced_by_milestone_index": index } },
                doc! { "$group": {
                    "_id": null,
                    "num_blocks": { "$count": {} },
                    "num_no_payload": { "$sum": {
                        // "$cond": [ { "block.payload": { "$ne": null } }, 1 , 0 ]
                        "$cond": [ { "$ne": [ "$block.payload", null ] }, 1 , 0 ]
                    } },
                } },
                // doc! { "$project": {
                //     "num_blocks": { "$toString": "$total_balance" },
                //     "num_no_payload": { "$toString": "$sig_locked_balance" },
                //     "num_tx_payload": { "$literal": ledger_index },
                //     "num_treasury_tx_payload": { "$literal": ledger_index },
                //     "num_tagged_data_payload": { "$literal": ledger_index },
                //     "num_milestone_payload": { "$literal": ledger_index },
                //     "num_confirmed": { "$literal": ledger_index },
                //     "num_conflicting": { "$literal": ledger_index },
                // } },
            ],
            None,
        )
        .await?
        .try_next()
        .await?
        .unwrap_or_default())
    }

}
