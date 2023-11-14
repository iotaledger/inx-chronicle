// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use iota_sdk::types::block::{
    output::OutputId, payload::signed_transaction::TransactionId, slot::SlotIndex, BlockId, SignedBlock,
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
        block_metadata::{BlockMetadata, BlockState, BlockWithMetadata},
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
    block: Raw<SignedBlock>,
    /// The block's metadata.
    metadata: BlockMetadata,
    /// The index of the slot to which this block commits.
    slot_index: SlotIndex,
    /// The block's payload type.
    payload_type: Option<u8>,
    /// Metadata about the possible transaction payload.
    transaction: Option<TransactionMetadata>,
}

impl From<BlockWithMetadata> for BlockDocument {
    fn from(BlockWithMetadata { block, metadata }: BlockWithMetadata) -> Self {
        let transaction = block
            .inner()
            .block()
            .as_basic_opt()
            .and_then(|b| b.payload())
            .and_then(|p| p.as_signed_transaction_opt())
            .map(|txn| TransactionMetadata {
                transaction_id: txn.transaction().id(),
                inputs: txn
                    .transaction()
                    .inputs()
                    .iter()
                    .map(|i| *i.as_utxo().output_id())
                    .collect(),
            });
        Self {
            block_id: metadata.block_id,
            slot_index: block.inner().slot_commitment_id().slot_index(),
            payload_type: block
                .inner()
                .block()
                .as_basic_opt()
                .and_then(|b| b.payload())
                .map(|p| p.kind()),
            block,
            metadata,
            transaction,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TransactionMetadata {
    transaction_id: TransactionId,
    inputs: Vec<OutputId>,
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
                            "transaction.transaction_id": { "$exists": true },
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
                .keys(doc! { "slot_index": -1, "metadata.block_state": 1 })
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
    pub block: SignedBlock,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IncludedBlockMetadataResult {
    #[serde(rename = "_id")]
    pub block_id: BlockId,
    pub metadata: BlockMetadata,
}

#[derive(Deserialize)]
struct RawResult {
    block: Raw<SignedBlock>,
}

/// Implements the queries for the core API.
impl BlockCollection {
    /// Get a [`SignedBlock`] by its [`BlockId`].
    pub async fn get_block(&self, block_id: &BlockId) -> Result<Option<SignedBlock>, DbError> {
        Ok(self.get_block_raw(block_id).await?.map(|raw| raw.into_inner()))
    }

    /// Get the raw bytes of a [`SignedBlock`] by its [`BlockId`].
    pub async fn get_block_raw(&self, block_id: &BlockId) -> Result<Option<Raw<SignedBlock>>, DbError> {
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

    /// Get the metadata of a [`SignedBlock`] by its [`BlockId`].
    pub async fn get_block_metadata(&self, block_id: &BlockId) -> Result<Option<BlockMetadata>, DbError> {
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

    /// Get the accepted blocks from a slot.
    pub async fn get_accepted_blocks(
        &self,
        index: SlotIndex,
    ) -> Result<impl Stream<Item = Result<BlockWithMetadata, DbError>>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "slot_index": index.0,
                        "metadata.block_state": BlockState::Confirmed.to_bson()
                    } },
                    doc! { "$sort": { "_id": 1 } },
                    doc! { "$project": {
                        "block": 1,
                        "metadata": 1
                    } },
                ],
                None,
            )
            .await?
            .map_err(Into::into))
    }

    /// Inserts [`SignedBlock`]s together with their associated [`BlockMetadata`].
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

    /// Finds the [`SignedBlock`] that included a transaction by [`TransactionId`].
    pub async fn get_block_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<IncludedBlockResult>, DbError> {
        #[derive(Deserialize)]
        struct Res {
            #[serde(rename = "_id")]
            block_id: BlockId,
            block: Raw<SignedBlock>,
        }

        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.block_state": BlockState::Finalized.to_bson(),
                        "transaction.transaction_id": transaction_id.to_bson(),
                    } },
                    doc! { "$project": { "block_id": "$_id", "block": 1 } },
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
    ) -> Result<Option<Raw<SignedBlock>>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.block_state": BlockState::Finalized.to_bson(),
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
    ) -> Result<Option<IncludedBlockMetadataResult>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.block_state": BlockState::Finalized.to_bson(),
                        "transaction.transaction_id": transaction_id.to_bson(),
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

    /// Gets the block containing the spending transaction of an output by [`OutputId`].
    pub async fn get_spending_transaction(&self, output_id: &OutputId) -> Result<Option<SignedBlock>, DbError> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.block_state": BlockState::Finalized.to_bson(),
                        "inputs.output_id": output_id.to_bson(),
                    } },
                    doc! { "$project": { "block": 1 } },
                ],
                None,
            )
            .await?
            .map_ok(|RawResult { block }| block.into_inner())
            .try_next()
            .await?)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct BlocksBySlotResult {
    #[serde(rename = "_id")]
    pub block_id: BlockId,
    pub payload_type: Option<u8>,
}

impl BlockCollection {
    /// Get the blocks in a slot by index as a stream of [`BlockId`]s.
    pub async fn get_blocks_by_slot_index(
        &self,
        slot_index: SlotIndex,
        page_size: usize,
        cursor: Option<BlockId>,
        sort: SortOrder,
    ) -> Result<impl Stream<Item = Result<BlocksBySlotResult, DbError>>, DbError> {
        let (sort, cmp) = match sort {
            SortOrder::Newest => (doc! {"slot_index": -1 }, "$lte"),
            SortOrder::Oldest => (doc! {"slot_index": 1 }, "$gte"),
        };

        let mut queries = vec![doc! { "slot_index": slot_index.0 }];
        if let Some(block_id) = cursor {
            queries.push(doc! { "_id": { cmp: block_id.to_bson() } });
        }

        Ok(self
            .aggregate(
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
            .map_err(Into::into))
    }
}
