// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, StreamExt, TryStreamExt};
use iota_sdk::types::{
    api::core::BlockState,
    block::{payload::signed_transaction::TransactionId, slot::SlotIndex, Block, BlockId},
};
use mongodb::{
    bson::doc,
    options::{IndexOptions, InsertManyOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::SortOrder;
use crate::{
    db::{
        mongodb::{DbError, InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::{
        block_metadata::{BlockMetadata, BlockWithMetadata, BlockWithTransactionMetadata, TransactionMetadata},
        raw::Raw,
        SerializeToBson,
    },
};

/// Chronicle Block record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockDocument {
    #[serde(rename = "_id")]
    block_id: BlockId,
    /// The block.
    block: Raw<Block>,
    /// The block's state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    block_state: Option<BlockState>,
    /// The index of the slot to which this block commits.
    slot_index: SlotIndex,
    /// The block's payload type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    payload_type: Option<u8>,
    /// Metadata about the possible transaction payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    transaction: Option<TransactionMetadata>,
}

impl From<BlockWithTransactionMetadata> for BlockDocument {
    fn from(
        BlockWithTransactionMetadata {
            block: BlockWithMetadata { metadata, block },
            transaction,
        }: BlockWithTransactionMetadata,
    ) -> Self {
        Self {
            block_id: metadata.block_id,
            slot_index: block.inner().slot_commitment_id().slot_index(),
            payload_type: block
                .inner()
                .body()
                .as_basic_opt()
                .and_then(|b| b.payload())
                .map(|p| p.kind()),
            block,
            block_state: metadata.block_state,
            transaction,
        }
    }
}

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
                .keys(doc! { "transaction.transaction_id": 1 })
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .name("transaction_id_index".to_string())
                        .partial_filter_expression(doc! {
                            "transaction": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "slot_index": -1 })
                .options(
                    IndexOptions::builder()
                        .name("block_slot_index_comp".to_string())
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
    pub block: Block,
}

#[derive(Deserialize)]
struct RawResult {
    block: Raw<Block>,
}

/// Implements the queries for the core API.
impl BlockCollection {
    /// Get a [`Block`] by its [`BlockId`].
    pub async fn get_block(&self, block_id: &BlockId) -> Result<Option<Block>, DbError> {
        Ok(self.get_block_raw(block_id).await?.map(|raw| raw.into_inner()))
    }

    /// Get the raw bytes of a [`Block`] by its [`BlockId`].
    pub async fn get_block_raw(&self, block_id: &BlockId) -> Result<Option<Raw<Block>>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "_id": block_id.to_bson() } },
                    doc! { "$project": { "block": 1 } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|RawResult { block }| block))
    }

    /// Get the metadata of a [`Block`] by its [`BlockId`].
    pub async fn get_block_metadata(&self, block_id: &BlockId) -> Result<Option<BlockMetadata>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "_id": block_id.to_bson() } },
                    doc! { "$project": {
                        "block_id": "$_id",
                        "block_state": 1,
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?)
    }

    /// Get the blocks from a slot.
    pub async fn get_blocks_by_slot(
        &self,
        SlotIndex(index): SlotIndex,
    ) -> Result<impl Stream<Item = Result<BlockWithMetadata, DbError>>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "slot_index": index } },
                    doc! { "$project": {
                        "block": 1,
                        "metadata": {
                            "block_id": "$_id",
                            "block_state": 1,
                        }
                    } },
                ],
                None,
            )
            .await?
            .map_err(Into::into))
    }

    /// Inserts [`Block`]s together with their associated [`BlockMetadata`].
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_blocks_with_metadata<I>(&self, blocks_with_metadata: I) -> Result<(), DbError>
    where
        I: IntoIterator<Item = BlockWithTransactionMetadata>,
        I::IntoIter: Send + Sync,
    {
        let docs = blocks_with_metadata.into_iter().map(BlockDocument::from);

        self.insert_many_ignore_duplicates(docs, InsertManyOptions::builder().ordered(false).build())
            .await?;

        Ok(())
    }

    /// Finds the [`Block`] that included a transaction by [`TransactionId`].
    pub async fn get_block_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<IncludedBlockResult>, DbError> {
        #[derive(Deserialize)]
        struct Res {
            #[serde(rename = "_id")]
            block_id: BlockId,
            block: Raw<Block>,
        }

        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "transaction": { "$exists": true },
                        "transaction.transaction_id": transaction_id.to_bson(),
                    } },
                    doc! { "$project": {
                        "_id": 1,
                        "block": 1,
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|Res { block_id, block }| IncludedBlockResult {
                block_id,
                block: block.into_inner(),
            }))
    }

    /// Finds the raw bytes of the block that included a transaction by [`TransactionId`].
    pub async fn get_block_raw_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<Raw<Block>>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "transaction": { "$exists": true },
                        "transaction.transaction_id": transaction_id.to_bson(),
                    } },
                    doc! { "$project": { "block": 1 } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|RawResult { block }| block))
    }

    /// Finds the block metadata that included a transaction by [`TransactionId`].
    pub async fn get_block_metadata_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<BlockMetadata>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "transaction": { "$exists": true },
                        "transaction.transaction_id": transaction_id.to_bson(),
                    } },
                    doc! { "$project": {
                        "block_id": "$_id",
                        "block_state": 1,
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?)
    }

    /// Finds the [`TransactionMetadata`] by [`TransactionId`].
    pub async fn get_transaction_metadata(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<TransactionMetadata>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "transaction": { "$exists": true },
                        "transaction.transaction_id": transaction_id.to_bson(),
                    } },
                    doc! { "$replaceWith": "$transaction" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?)
    }
}

#[allow(missing_docs)]
pub struct BlocksBySlotResult<S> {
    pub count: usize,
    pub stream: S,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct BlockResult {
    #[serde(rename = "_id")]
    pub block_id: BlockId,
    pub payload_type: Option<u8>,
}

impl BlockCollection {
    /// Get the blocks in a slot by index as a stream of [`BlockId`]s.
    pub async fn get_blocks_by_slot_index(
        &self,
        SlotIndex(slot_index): SlotIndex,
        page_size: usize,
        cursor: Option<BlockId>,
        sort: SortOrder,
    ) -> Result<BlocksBySlotResult<impl Stream<Item = Result<BlockResult, DbError>>>, DbError> {
        let (sort, cmp) = match sort {
            SortOrder::Newest => (doc! {"slot_index": -1 }, "$lte"),
            SortOrder::Oldest => (doc! {"slot_index": 1 }, "$gte"),
        };

        let mut queries = vec![doc! { "slot_index": slot_index }];
        if let Some(block_id) = cursor {
            queries.push(doc! { "_id": { cmp: block_id.to_bson() } });
        }

        let count = self
            .collection()
            .find(doc! { "slot_index": slot_index }, None)
            .await?
            .count()
            .await;

        Ok(BlocksBySlotResult {
            count,
            stream: self
                .aggregate::<BlockResult>(
                    [
                        doc! { "$match": { "$and": queries } },
                        doc! { "$sort": sort },
                        doc! { "$limit": page_size as i64 },
                        doc! { "$project": {
                            "_id": 1,
                            "payload_type": 1,
                        } },
                    ],
                    None,
                )
                .await?
                .map_err(Into::into),
        })
    }
}
