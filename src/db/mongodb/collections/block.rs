// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use iota_sdk::types::{
    api::core::{BlockMetadataResponse, BlockState},
    block::{
        output::OutputId, payload::signed_transaction::TransactionId, slot::SlotIndex, BlockId, SignedBlock,
        SignedBlockDto,
    },
    TryFromDto,
};
use mongodb::{
    bson::doc,
    options::{IndexOptions, InsertManyOptions},
    IndexModel,
};
use packable::PackableExt;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::SortOrder;
use crate::{
    db::{
        mongodb::{DbError, InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::SerializeToBson,
};

/// Chronicle Block record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockDocument {
    #[serde(rename = "_id")]
    block_id: BlockId,
    /// The block.
    block: SignedBlockDto,
    /// The raw bytes of the block.
    #[serde(with = "serde_bytes")]
    raw: Vec<u8>,
    /// The block's metadata.
    metadata: BlockMetadataResponse,
}

// impl From<BlockData> for BlockDocument {
//     fn from(
//         BlockData {
//             block_id,
//             block,
//             raw,
//             metadata,
//         }: BlockData,
//     ) -> Self { Self { block_id, block, raw, metadata, }
//     }
// }

// impl From<(BlockId, Block, Vec<u8>, BlockMetadata)> for BlockDocument {
//     fn from((block_id, block, raw, metadata): (BlockId, Block, Vec<u8>, BlockMetadata)) -> Self {
//         Self {
//             block_id,
//             block,
//             raw,
//             metadata,
//         }
//     }
// }

/// The iota blocks collection.
pub struct BlockCollection {
    collection: mongodb::Collection<BlockDocument>,
}

#[async_trait::async_trait]
impl MongoDbCollection for BlockCollection {
    const NAME: &'static str = "iota_blocks";
    type Document = BlockDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), DbError> {
        self.create_index(
            IndexModel::builder()
                .keys(doc! { "block.payload.transaction_id": 1 })
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .name("transaction_id_index".to_string())
                        .partial_filter_expression(doc! {
                            "block.payload.transaction_id": { "$exists": true },
                            "metadata.block_state": { "$eq": BlockState::Finalized.to_bson() },
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

#[derive(Debug, Clone)]
pub struct IncludedBlockResult {
    pub block_id: BlockId,
    pub block: SignedBlock,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IncludedBlockMetadataResult {
    #[serde(rename = "_id")]
    pub block_id: BlockId,
    pub metadata: BlockMetadataResponse,
}

#[derive(Deserialize)]
struct RawResult {
    #[serde(with = "serde_bytes")]
    raw: Vec<u8>,
}

#[derive(Deserialize)]
struct BlockIdResult {
    #[serde(rename = "_id")]
    block_id: BlockId,
}

/// Implements the queries for the core API.
impl BlockCollection {
    /// Get a [`Block`] by its [`BlockId`].
    pub async fn get_block(&self, block_id: &BlockId) -> Result<Option<SignedBlock>, DbError> {
        Ok(self
            .get_block_raw(block_id)
            .await?
            .map(|raw| SignedBlock::unpack_unverified(raw).unwrap()))
    }

    /// Get the raw bytes of a [`Block`] by its [`BlockId`].
    pub async fn get_block_raw(&self, block_id: &BlockId) -> Result<Option<Vec<u8>>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "_id": block_id.to_bson() } },
                    doc! { "$project": { "raw": 1 } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|RawResult { raw }| raw))
    }

    /// Get the metadata of a [`Block`] by its [`BlockId`].
    pub async fn get_block_metadata(&self, block_id: &BlockId) -> Result<Option<BlockMetadataResponse>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "_id": block_id.to_bson() } },
                    doc! { "$replaceWith": "$metadata" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?)
    }

    // /// Get the children of a [`Block`] as a stream of [`BlockId`]s.
    // pub async fn get_block_children(
    //     &self,
    //     block_id: &BlockId,
    //     block_referenced_index: MilestoneIndex,
    //     below_max_depth: u8,
    //     page_size: usize,
    //     page: usize,
    // ) -> Result<impl Stream<Item = Result<BlockId, Error>>, Error> { let max_referenced_index =
    //   block_referenced_index + below_max_depth as u32;

    //     Ok(self
    //         .aggregate(
    //             [
    //                 doc! { "$match": {
    //                     "metadata.referenced_by_milestone_index": {
    //                         "$gte": block_referenced_index,
    //                         "$lte": max_referenced_index
    //                     },
    //                     "block.parents": block_id,
    //                 } },
    //                 doc! { "$sort": {"metadata.referenced_by_milestone_index": -1} },
    //                 doc! { "$skip": (page_size * page) as i64 },
    //                 doc! { "$limit": page_size as i64 },
    //                 doc! { "$project": { "_id": 1 } },
    //             ],
    //             None,
    //         )
    //         .await?
    //         .map_ok(|BlockIdResult { block_id }| block_id))
    // }

    // /// Get the blocks that were referenced by the specified milestone (in White-Flag order).
    // pub async fn get_referenced_blocks_in_white_flag_order(
    //     &self,
    //     index: MilestoneIndex,
    // ) -> Result<Vec<BlockId>, Error> { let block_ids = self .aggregate::<BlockIdResult>( [ doc! { "$match": {
    //   "metadata.referenced_by_milestone_index": index } }, doc! { "$sort": { "metadata.white_flag_index": 1 } }, doc!
    //   { "$project": { "_id": 1 } }, ], None, ) .await? .map_ok(|res| res.block_id) .try_collect() .await?;

    //     Ok(block_ids)
    // }

    // /// Get the blocks that were referenced by the specified milestone (in White-Flag order).
    // pub async fn get_referenced_blocks_in_white_flag_order_stream(
    //     &self,
    //     index: MilestoneIndex,
    // ) -> Result<impl Stream<Item = Result<(BlockId, Block, Vec<u8>, BlockMetadata), Error>>, Error> { #[derive(Debug,
    //   Deserialize)] struct QueryRes { #[serde(rename = "_id")] block_id: BlockId, #[serde(with = "serde_bytes")] raw:
    //   Vec<u8>, metadata: BlockMetadata, }

    //     Ok(self
    //         .aggregate::<QueryRes>(
    //             [
    //                 doc! { "$match": { "metadata.referenced_by_milestone_index": index } },
    //                 doc! { "$sort": { "metadata.white_flag_index": 1 } },
    //             ],
    //             None,
    //         )
    //         .await?
    //         .map_ok(|r| {
    //             (
    //                 r.block_id,
    //                 iota_sdk::types::block::Block::unpack_unverified(r.raw.clone())
    //                     .unwrap()
    //                     .into(),
    //                 r.raw,
    //                 r.metadata,
    //             )
    //         }))
    // }

    // /// Get the blocks that were applied by the specified milestone (in White-Flag order).
    // pub async fn get_applied_blocks_in_white_flag_order(&self, index: MilestoneIndex) -> Result<Vec<BlockId>, Error>
    // {     let block_ids = self
    //         .aggregate::<BlockIdResult>(
    //             [
    //                 doc! { "$match": {
    //                     "metadata.referenced_by_milestone_index": index,
    //                     "metadata.inclusion_state": LedgerInclusionState::Included,
    //                 } },
    //                 doc! { "$sort": { "metadata.white_flag_index": 1 } },
    //                 doc! { "$project": { "_id": 1 } },
    //             ],
    //             None,
    //         )
    //         .await?
    //         .map_ok(|res| res.block_id)
    //         .try_collect()
    //         .await?;

    //     Ok(block_ids)
    // }

    /// Inserts [`Block`]s together with their associated [`BlockMetadata`].
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_blocks_with_metadata<I, B>(&self, blocks_with_metadata: I) -> Result<(), DbError>
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
    ) -> Result<Option<IncludedBlockResult>, DbError> {
        #[derive(Deserialize)]
        struct IncludedBlockRes {
            #[serde(rename = "_id")]
            block_id: BlockId,
            block: SignedBlockDto,
        }

        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.block_state": BlockState::Finalized.to_bson(),
                        "block.payload.transaction_id": transaction_id.to_bson(),
                    } },
                    doc! { "$project": { "block_id": "$_id", "block": 1 } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|IncludedBlockRes { block_id, block }| IncludedBlockResult {
                block_id,
                block: SignedBlock::try_from_dto(block).unwrap(),
            }))
    }

    /// Finds the raw bytes of the block that included a transaction by [`TransactionId`].
    pub async fn get_block_raw_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<Vec<u8>>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.block_state": BlockState::Finalized.to_bson(),
                        "block.payload.transaction_id": transaction_id.to_bson(),
                    } },
                    doc! { "$project": { "raw": 1 } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|RawResult { raw }| raw))
    }

    /// Finds the block metadata that included a transaction by [`TransactionId`].
    pub async fn get_block_metadata_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<IncludedBlockMetadataResult>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.block_state": BlockState::Finalized.to_bson(),
                        "block.payload.transaction_id": transaction_id.to_bson(),
                    } },
                    doc! { "$project": {
                        "_id": 1,
                        "metadata": 1,
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?)
    }

    /// Gets the spending transaction of an [`Output`](crate::model::utxo::Output) by [`OutputId`].
    pub async fn get_spending_transaction(&self, output_id: &OutputId) -> Result<Option<SignedBlock>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.block_state": BlockState::Finalized.to_bson(),
                        "block.payload.essence.inputs.transaction_id": output_id.transaction_id().to_bson(),
                        "block.payload.essence.inputs.index": &(output_id.index() as i32)
                    } },
                    doc! { "$project": { "raw": 1 } },
                ],
                None,
            )
            .await?
            .map_ok(|RawResult { raw }| SignedBlock::unpack_unverified(raw).unwrap())
            .try_next()
            .await?)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct BlocksBySlotResult {
    #[serde(rename = "_id")]
    pub block_id: BlockId,
    pub payload_kind: Option<String>,
    pub issuing_time: u64,
}

impl BlockCollection {
    /// Get the [`Block`]s in a milestone by index as a stream of [`BlockId`]s.
    pub async fn get_blocks_by_slot_index(
        &self,
        slot_index: SlotIndex,
        page_size: usize,
        cursor: Option<u32>,
        sort: SortOrder,
    ) -> Result<impl Stream<Item = Result<BlocksBySlotResult, DbError>>, DbError> {
        let (sort, cmp) = match sort {
            SortOrder::Newest => (doc! {"block.issuing_time": -1 }, "$lte"),
            SortOrder::Oldest => (doc! {"block.issuing_time": 1 }, "$gte"),
        };

        let mut queries = vec![doc! { "block.latest_finalized_slot": slot_index.0 }];
        if let Some(issuing_time) = cursor {
            queries.push(doc! { "block.issuing_time": { cmp: issuing_time } });
        }

        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "$and": queries } },
                    doc! { "$sort": sort },
                    doc! { "$limit": page_size as i64 },
                    doc! { "$project": {
                        "_id": 1,
                        "payload_kind": "$block.payload.kind",
                        "issuing_time": "$block.issuing_time"
                    } },
                ],
                None,
            )
            .await?
            .map_err(Into::into))
    }
}
