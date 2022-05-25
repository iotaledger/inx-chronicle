// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, StreamExt, TryStreamExt};
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::FindOptions,
    results::UpdateResult,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        ledger::{BlockMetadata, LedgerInclusionState},
        stardust::block::{Address, Block, BlockId, TransactionId},
        tangle::MilestoneIndex,
    },
};

/// Chronicle Block record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockDocument {
    /// The id of the current block.
    #[serde(rename = "_id")]
    pub block_id: BlockId,
    /// The block.
    pub block: Block,
    /// The raw bytes of the block.
    #[serde(with = "serde_bytes")]
    pub raw: Vec<u8>,
    /// The block's metadata.
    // TODO: Remove `Option`
    pub metadata: Option<BlockMetadata>,
}

impl BlockDocument {
    /// The stardust blocks collection name.
    pub const COLLECTION: &'static str = "stardust_blocks";
}

/// A single transaction history result row.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionHistoryResult {
    /// The transaction id.
    pub transaction_id: TransactionId,
    /// The index of the output that this transfer represents.
    pub output_index: u16,
    /// Whether this is a spent or unspent output.
    pub is_spent: bool,
    /// The inclusion state of the output's transaction.
    pub inclusion_state: Option<LedgerInclusionState>,
    /// The transaction's block id.
    pub block_id: BlockId,
    /// The milestone index that references the transaction.
    pub milestone_index: Option<MilestoneIndex>,
    /// The transfer amount.
    pub amount: u64,
}

/// Implements the queries for the core API.
impl MongoDb {
    /// Get a [`Block`] by its [`BlockId`].
    pub async fn get_block(&self, block_id: &BlockId) -> Result<Option<Block>, Error> {
        self.0
            .collection::<Block>(BlockDocument::COLLECTION)
            .find_one(doc! {"_id": bson::to_bson(block_id)?}, Self::projection("block"))
            .await
    }

    /// Get the raw bytes of a [`Block`] by its [`BlockId`].
    pub async fn get_block_raw(&self, block_id: &BlockId) -> Result<Option<Vec<u8>>, Error> {
        self.0
            .collection::<Vec<u8>>(BlockDocument::COLLECTION)
            .find_one(doc! {"_id": bson::to_bson(block_id)?}, Self::projection("raw"))
            .await
    }

    /// Get the metadata of a [`Block`] by its [`BlockId`].
    pub async fn get_block_metadata(&self, block_id: &BlockId) -> Result<Option<BlockMetadata>, Error> {
        self.0
            .collection::<BlockMetadata>(BlockDocument::COLLECTION)
            .find_one(doc! {"_id": bson::to_bson(block_id)?}, Self::projection("metadata"))
            .await
    }

    /// Get the children of a [`Block`] as a stream of [`BlockId`]s.
    pub async fn get_block_children(
        &self,
        block_id: &BlockId,
        page_size: usize,
        page: usize,
    ) -> Result<impl Stream<Item = Result<BlockId, Error>>, Error> {
        self.0
            .collection::<BlockId>(BlockDocument::COLLECTION)
            .find(
                doc! {"block.parents": bson::to_bson(block_id)?},
                FindOptions::builder()
                    .skip((page_size * page) as u64)
                    .sort(doc! {"metadata.referenced_by_milestone_index": -1})
                    .limit(page_size as i64)
                    .projection(doc! {"block_id": 1 })
                    .build(),
            )
            .await
    }

    /// Upserts a [`BlockRecord`] to the database.
    #[deprecated(note = "Use `insert_block_with_metadata` instead")]
    pub async fn upsert_block_record(&self, _block_record: &BlockDocument) -> Result<UpdateResult, Error> {
        unimplemented!();
    }

    /// Inserts a [`Block`] together with its associated [`BlockMetadata`].
    pub async fn insert_block_with_metadata(
        &self,
        block_id: BlockId,
        block: Block,
        raw: Vec<u8>,
        metadata: BlockMetadata,
    ) -> Result<(), Error> {
        let block_document = BlockDocument {
            block_id,
            block,
            raw,
            metadata: Some(metadata),
        };

        let _ = self
            .0
            .collection::<BlockDocument>(BlockDocument::COLLECTION)
            .insert_one(block_document, None)
            .await;

        Ok(())
    }

    /// Updates a [`BlockRecord`] with [`Metadata`].
    #[deprecated(note = "We never update block metadata")]
    pub async fn update_block_metadata(
        &self,
        block_id: &BlockId,
        metadata: &BlockMetadata,
    ) -> Result<UpdateResult, Error> {
        self.0
            .collection::<BlockDocument>(BlockDocument::COLLECTION)
            .update_one(
                doc! { "_id": bson::to_bson(block_id)? },
                doc! { "$set": { "metadata": bson::to_document(metadata)? } },
                None,
            )
            .await
    }

    /// Aggregates the spending transactions
    #[deprecated(note = "don't use")]
    pub async fn get_spending_transaction(
        &self,
        transaction_id: &TransactionId,
        idx: u16,
    ) -> Result<Option<BlockDocument>, Error> {
        self.0
            .collection::<BlockDocument>(BlockDocument::COLLECTION)
            .find_one(
                doc! {
                    "inclusion_state": bson::to_bson(&LedgerInclusionState::Included)?,
                    "block.payload.essence.inputs.transaction_id": bson::to_bson(transaction_id)?,
                    "block.payload.essence.inputs.index": bson::to_bson(&idx)?
                },
                None,
            )
            .await
    }

    /// Finds the [`Block`] that included a transaction by [`TransactionId`].
    pub async fn get_block_for_transaction(&self, transaction_id: &TransactionId) -> Result<Option<Block>, Error> {
        self.0
            .collection::<Block>(BlockDocument::COLLECTION)
            .find_one(
                doc! {
                    "inclusion_state": bson::to_bson(&LedgerInclusionState::Included)?,
                    "block.payload.transaction_id": bson::to_bson(transaction_id)?,
                },
                Self::projection("block"),
            )
            .await
    }

    /// Aggregates the transaction history for an address.
    pub async fn get_transaction_history(
        &self,
        address: &Address,
        page_size: usize,
        page: usize,
        start_milestone: MilestoneIndex,
        end_milestone: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<TransactionHistoryResult, Error>>, Error> {
        self.0
        .collection::<BlockDocument>(BlockDocument::COLLECTION)
        .aggregate(vec![
            // Only outputs for this address
            doc! { "$match": {
                "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                "inclusion_state": bson::to_bson(&LedgerInclusionState::Included)?, 
                "block.payload.essence.outputs.unlocks": bson::to_bson(&address)?
            } },
            doc! { "$set": {
                "block.payload.essence.outputs": {
                    "$filter": {
                        "input": "$block.payload.essence.outputs",
                        "as": "output",
                        "cond": { "$eq": [ "$$output.unlock_conditions", bson::to_bson(&address)? ] }
                    }
                }
            } },
            // One result per output
            doc! { "$unwind": { "path": "$block.payload.essence.outputs", "includeArrayIndex": "block.payload.essence.outputs.idx" } },
            // Lookup spending inputs for each output, if they exist
            doc! { "$lookup": {
                "from": "stardust_blocks",
                // Keep track of the output id
                "let": { "transaction_id": "$block.payload.transaction_id", "index": "$block.payload.essence.outputs.idx" },
                "pipeline": [
                    // Match using the output's index
                    { "$match": { 
                        "inclusion_state": bson::to_bson(&LedgerInclusionState::Included)?, 
                        "block.payload.essence.inputs.transaction_id": "$$transaction_id",
                        "block.payload.essence.inputs.index": "$$index"
                    } },
                    { "$set": {
                        "block.payload.essence.inputs": {
                            "$filter": {
                                "input": "$block.payload.essence.inputs",
                                "as": "input",
                                "cond": { "$and": {
                                    "$eq": [ "$$input.transaction_id", "$$transaction_id" ],
                                    "$eq": [ "$$input.index", "$$index" ],
                                } }
                            }
                        }
                    } },
                    // One result per spending input
                    { "$unwind": { "path": "$block.payload.essence.outputs", "includeArrayIndex": "block.payload.essence.outputs.idx" } },
                ],
                // Store the result
                "as": "spending_transaction"
            } },
            // Add a null spending transaction so that unwind will create two records
            doc! { "$set": { "spending_transaction": { "$concatArrays": [ "$spending_transaction", [ null ] ] } } },
            // Unwind the outputs into one or two results
            doc! { "$unwind": { "path": "$spending_transaction", "preserveNullAndEmptyArrays": true } },
            // Project the result
            doc! { "$project": {
                "transaction_id": "$block.payload.transaction_id",
                "output_idx": "$block.payload.essence.outputs.idx",
                "is_spent": { "$ne": [ "$spending_transaction", null ] },
                "inclusion_state": "$metadata.inclusion_state",
                "block_id": "$block.id",
                "milestone_index": "$metadata.referenced_by_milestone_index",
                "amount": "$block.payload.essence.outputs.amount",
            } },
            doc! { "$sort": { "metadata.referenced_by_milestone_index": -1 } },
            doc! { "$skip": (page_size * page) as i64 },
            doc! { "$limit": page_size as i64 },
        ], None)
        .await
        .map(|c| c.then(|r| async { Ok(bson::from_document(r?)?) }))
    }
}

/// Address analytics result.
#[cfg(feature = "analytics")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressAnalyticsResult {
    /// The number of addresses used in the time period.
    pub total_addresses: u64,
    /// The number of addresses that received tokens in the time period.
    pub recv_addresses: u64,
    /// The number of addresses that sent tokens in the time period.
    pub send_addresses: u64,
}

#[cfg(feature = "analytics")]
impl MongoDb {
    /// Create aggregate statistics of all addresses.
    pub async fn aggregate_addresses(
        &self,
        start_milestone: MilestoneIndex,
        end_milestone: MilestoneIndex,
    ) -> Result<Option<AddressAnalyticsResult>, Error> {
        Ok(self.0.collection::<BlockDocument>(BlockDocument::COLLECTION)
        .aggregate(
            vec![
                doc! { "$match": {
                    "inclusion_state": bson::to_bson(&LedgerInclusionState::Included)?,
                    "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                    "block.payload.kind": "transaction",
                } },
                doc! { "$unwind": { "path": "$block.payload.essence.inputs", "includeArrayIndex": "block.payload.essence.inputs.idx" } },
                doc! { "$lookup": {
                    "from": "stardust_blocks",
                    "let": { "transaction_id": "$block.payload.essence.inputs.transaction_id", "index": "$block.payload.essence.inputs.index" },
                    "pipeline": [
                        { "$match": { 
                            "inclusion_state": bson::to_bson(&LedgerInclusionState::Included)?, 
                            "block.payload.transaction_id": "$$transaction_id",
                        } },
                        { "$set": {
                            "block.payload.essence.outputs": {
                                "$arrayElemAt": [
                                    "$block.payload.essence.outputs",
                                    "$$index"
                                ]
                            }
                        } },
                    ],
                    "as": "spent_transaction"
                } },
                doc! { "$set": { "send_address": "$spent_transaction.block.payload.essence.outputs.unlocks" } },
                doc! { "$unwind": { "path": "$block.payload.essence.outputs", "includeArrayIndex": "block.payload.essence.outputs.idx" } },
                doc! { "$set": { "recv_address": "$block.payload.essence.outputs.unlocks" } },
                doc! { "$facet": {
                    "total": [
                        { "$set": { "address": ["$send_address", "$recv_address"] } },
                        { "$unwind": { "path": "$address" } },
                        { "$group" : {
                            "_id": "$address",
                            "addresses": { "$count": { } }
                        }},
                    ],
                    "recv": [
                        { "$group" : {
                            "_id": "$recv_address",
                            "addresses": { "$count": { } }
                        }},
                    ],
                    "send": [
                        { "$group" : {
                            "_id": "$send_address",
                            "addresses": { "$count": { } }
                        }},
                    ],
                } },
                doc! { "$project": {
                    "total_addresses": { "$arrayElemAt": ["$total.addresses", 0] },
                    "recv_addresses": { "$arrayElemAt": ["$recv.addresses", 0] },
                    "send_addresses": { "$arrayElemAt": ["$send.addresses", 0] },
                } },
            ],
            None,
        )
        .await?
        .try_next()
        .await?
        .map(bson::from_document)
        .transpose()?)
    }
}
