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
    tangle::BlockData,
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

impl From<BlockData> for BlockDocument {
    fn from(
        BlockData {
            block_id,
            block,
            raw,
            metadata,
        }: BlockData,
    ) -> Self {
        Self {
            block_id,
            block,
            raw,
            metadata,
        }
    }
}

impl From<(BlockId, Block, Vec<u8>, BlockMetadata)> for BlockDocument {
    fn from((block_id, block, raw, metadata): (BlockId, Block, Vec<u8>, BlockMetadata)) -> Self {
        Self {
            block_id,
            block,
            raw,
            metadata,
        }
    }
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
                .keys(doc! { "metadata.referenced_by_milestone_index": -1, "metadata.white_flag_index": 1, "metadata.inclusion_state": 1 })
                .options(
                    IndexOptions::builder()
                        .name("block_referenced_index_comp".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct IncludedBlockResult {
    pub block_id: BlockId,
    pub block: Block,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IncludedBlockMetadataResult {
    pub block_id: BlockId,
    pub metadata: BlockMetadata,
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

    /// Get the blocks that were referenced by the specified milestone (in White-Flag order).
    pub async fn get_referenced_blocks_in_white_flag_order(
        &self,
        index: MilestoneIndex,
    ) -> Result<Vec<BlockId>, Error> {
        #[derive(Deserialize)]
        struct BlockIdResult {
            id: BlockId,
        }

        let block_ids = self
            .aggregate::<BlockIdResult>(
                vec![
                    doc! { "$match": { "metadata.referenced_by_milestone_index": index } },
                    doc! { "$sort": { "metadata.white_flag_index": 1 } },
                    doc! { "$replaceWith": { "id": "$_id" } },
                ],
                None,
            )
            .await?
            .map_ok(|res| res.id)
            .try_collect()
            .await?;

        Ok(block_ids)
    }

    /// Get the blocks that were referenced by the specified milestone (in White-Flag order).
    pub async fn get_referenced_blocks_in_white_flag_order_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<(BlockId, Block, Vec<u8>, BlockMetadata), Error>>, Error> {
        Ok(self
            .aggregate::<BlockDocument>(
                vec![
                    doc! { "$match": { "metadata.referenced_by_milestone_index": index } },
                    doc! { "$sort": { "metadata.white_flag_index": 1 } },
                ],
                None,
            )
            .await?
            .map_ok(
                |BlockDocument {
                     block_id,
                     block,
                     raw,
                     metadata,
                 }| (block_id, block, raw, metadata),
            ))
    }

    /// Get the blocks that were applied by the specified milestone (in White-Flag order).
    pub async fn get_applied_blocks_in_white_flag_order(&self, index: MilestoneIndex) -> Result<Vec<BlockId>, Error> {
        #[derive(Deserialize)]
        struct BlockIdResult {
            id: BlockId,
        }

        let block_ids = self
            .aggregate::<BlockIdResult>(
                vec![
                    doc! { "$match": {
                        "metadata.referenced_by_milestone_index": index,
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                    } },
                    doc! { "$sort": { "metadata.white_flag_index": 1 } },
                    doc! { "$replaceWith": { "id": "$_id" } },
                ],
                None,
            )
            .await?
            .map_ok(|res| res.id)
            .try_collect()
            .await?;

        Ok(block_ids)
    }

    /// Inserts [`Block`]s together with their associated [`BlockMetadata`].
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_blocks_with_metadata<I, B>(&self, blocks_with_metadata: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = B>,
        I::IntoIter: Send + Sync,
        BlockDocument: From<B>,
    {
        let blocks_with_metadata = blocks_with_metadata.into_iter().map(BlockDocument::from);

        self.insert_many_ignore_duplicates(
            blocks_with_metadata,
            InsertManyOptions::builder().ordered(false).build(),
        )
        .await?;

        Ok(())
    }

    /// Finds the [`Block`] that included a transaction by [`TransactionId`].
    pub async fn get_block_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<IncludedBlockResult>, Error> {
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
                doc! { "$replaceWith": {
                    "block_id": "$_id",
                    "block": "$block",
                } },
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

    /// Finds the [`BlockMetadata`] that included a transaction by [`TransactionId`].
    pub async fn get_block_metadata_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<IncludedBlockMetadataResult>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": {
                    "metadata.inclusion_state": LedgerInclusionState::Included,
                    "block.payload.transaction_id": transaction_id,
                } },
                doc! { "$replaceWith": {
                    "block_id": "$_id",
                    "metadata": "$metadata",
                } },
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
