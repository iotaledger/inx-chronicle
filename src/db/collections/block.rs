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

use super::SortOrder;
use crate::{
    db::{
        collections::OutputCollection,
        mongodb::{InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt},
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

#[async_trait::async_trait]
impl MongoDbCollection for BlockCollection {
    const NAME: &'static str = "stardust_blocks";
    type Document = BlockDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), Error> {
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

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "metadata.referenced_by_milestone_index": -1 })
                .options(
                    IndexOptions::builder()
                        .name("block_referenced_index".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}

/// Implements the queries for the core API.
impl BlockCollection {
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
                        { "$sort": { "_id": 1 } },
                        { "$replaceWith": "$output" }
                    ],
                    "as": "block.payload.essence.outputs"
                } },
                // Stupidly, if the block has no payload, then the above lookup
                // will add the structure, causing the deserialization to fail.
                // So this is needed to make sure we remove it if necessary.
                doc! { "$set": { "block.payload": { "$cond": [
                    { "$not": [ "$block.payload.kind" ] },
                    "$$REMOVE",
                    "$block.payload",
                ] } } },
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
                        { "$sort": { "_id": 1 } },
                        { "$replaceWith": "$output" }
                    ],
                    "as": "block.payload.essence.outputs"
                } },
                // Stupidly, if the block has no payload, then the above lookup
                // will add the structure, causing the deserialization to fail.
                // So this is needed to make sure we remove it if necessary.
                doc! { "$set": { "block.payload": { "$cond": [
                    { "$not": [ "$block.payload.kind" ] },
                    "$$REMOVE",
                    "$block.payload",
                ] } } },
                doc! { "$replaceWith": "$block" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Finds the raw bytes of the block that included a transaction by [`TransactionId`].
    pub async fn get_block_raw_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<Vec<u8>>, Error> {
        #[derive(Deserialize)]
        struct RawResult {
            #[serde(with = "serde_bytes")]
            data: Vec<u8>,
        }

        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                        "block.payload.transaction_id": transaction_id,
                    } },
                    doc! { "$replaceWith": { "data": "$raw" } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|RawResult { data }| data))
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
                        { "$sort": { "_id": 1 } },
                        { "$replaceWith": "$output" }
                    ],
                    "as": "block.payload.essence.outputs"
                } },
                // Stupidly, if the block has no payload, then the above lookup
                // will add the structure, causing the deserialization to fail.
                // So this is needed to make sure we remove it if necessary.
                doc! { "$set": { "block.payload": { "$cond": [
                    { "$not": [ "$block.payload.kind" ] },
                    "$$REMOVE",
                    "$block.payload",
                ] } } },
                doc! { "$replaceWith": "$block" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct BlocksByMilestoneResult {
    pub block_id: BlockId,
    pub white_flag_index: u32,
}

impl BlockCollection {
    /// Get the [`Block`]s in a milestone by index as a stream of [`BlockId`]s.
    pub async fn get_blocks_by_milestone_index(
        &self,
        milestone_index: MilestoneIndex,
        page_size: usize,
        cursor: Option<u32>,
        sort: SortOrder,
    ) -> Result<impl Stream<Item = Result<BlocksByMilestoneResult, Error>>, Error> {
        let (sort, cmp) = match sort {
            SortOrder::Newest => (doc! {"metadata.white_flag_index": -1 }, "$lte"),
            SortOrder::Oldest => (doc! {"metadata.white_flag_index": 1 }, "$gte"),
        };

        let mut queries = vec![doc! { "metadata.referenced_by_milestone_index": milestone_index }];
        if let Some(white_flag_index) = cursor {
            queries.push(doc! { "metadata.white_flag_index": { cmp: white_flag_index } });
        }

        self.aggregate(
            vec![
                doc! { "$match": { "$and": queries } },
                doc! { "$sort": sort },
                doc! { "$limit": page_size as i64 },
                doc! { "$replaceWith": {
                    "block_id": "$_id",
                    "white_flag_index": "$metadata.white_flag_index"
                } },
            ],
            None,
        )
        .await
    }
}
