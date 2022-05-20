// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::{self, doc, Document},
    error::Error,
    options::{FindOptions, UpdateOptions},
    results::UpdateResult,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        ledger::{LedgerInclusionState, Metadata},
        stardust::block::{Address, Block, BlockId, Output, TransactionId},
    },
};

/// Chronicle Block record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockRecord {
    /// The block.
    pub block: Block,
    /// The raw bytes of the block.
    #[serde(with = "serde_bytes")]
    pub raw: Vec<u8>,
    /// The block's metadata.
    pub metadata: Option<Metadata>,
}

impl BlockRecord {
    /// The stardust blocks collection name.
    pub const COLLECTION: &'static str = "stardust_blocks";

    /// Creates a new block record.
    pub fn new(block: Block, raw: Vec<u8>) -> Self {
        Self {
            block,
            raw,
            metadata: None,
        }
    }
}

#[cfg(feature = "inx")]
impl TryFrom<inx::proto::Block> for BlockRecord {
    type Error = inx::Error;

    fn try_from(value: inx::proto::Block) -> Result<Self, Self::Error> {
        let (block, raw_block) = value.try_into()?;
        Ok(Self::new(block.block.into(), raw_block))
    }
}

#[cfg(feature = "inx")]
impl TryFrom<(inx::proto::RawBlock, inx::proto::BlockMetadata)> for BlockRecord {
    type Error = inx::Error;

    fn try_from(
        (raw_block, metadata): (inx::proto::RawBlock, inx::proto::BlockMetadata),
    ) -> Result<Self, Self::Error> {
        let block = bee_block_stardust::Block::try_from(raw_block.clone())?;
        Ok(Self {
            block: block.into(),
            raw: raw_block.data,
            metadata: Some(inx::BlockMetadata::try_from(metadata)?.into()),
        })
    }
}

/// A result received when querying for a single output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputResult {
    /// The id of the block this output came from.
    pub block_id: BlockId,
    /// The metadata of the block this output came from.
    pub metadata: Option<Metadata>,
    /// The output.
    pub output: Output,
}

impl MongoDb {
    /// Get milestone with index.
    pub async fn get_block(&self, block_id: &BlockId) -> Result<Option<BlockRecord>, Error> {
        self.0
            .collection::<BlockRecord>(BlockRecord::COLLECTION)
            .find_one(doc! {"block.id": bson::to_bson(block_id)?}, None)
            .await
    }

    /// Get the children of a block.
    pub async fn get_block_children(
        &self,
        block_id: &BlockId,
        page_size: usize,
        page: usize,
    ) -> Result<impl Stream<Item = Result<BlockRecord, Error>>, Error> {
        self.0
            .collection::<BlockRecord>(BlockRecord::COLLECTION)
            .find(
                doc! {"block.parents": bson::to_bson(block_id)?},
                FindOptions::builder()
                    .skip((page_size * page) as u64)
                    .sort(doc! {"milestone_index": -1})
                    .limit(page_size as i64)
                    .build(),
            )
            .await
    }

    /// Upserts a [`BlockRecord`] to the database.
    pub async fn upsert_block_record(&self, block_record: &BlockRecord) -> Result<UpdateResult, Error> {
        self.0
            .collection::<BlockRecord>(BlockRecord::COLLECTION)
            .update_one(
                doc! { "_id": bson::to_bson(&block_record.block.id)? },
                doc! { "$set": bson::to_document(block_record)? },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
    }

    /// Updates a [`BlockRecord`] with [`Metadata`].
    pub async fn update_block_metadata(
        &self,
        block_id: &BlockId,
        metadata: &Metadata,
    ) -> Result<UpdateResult, Error> {
        self.0
            .collection::<BlockRecord>(BlockRecord::COLLECTION)
            .update_one(
                doc! { "block.id": bson::to_bson(block_id)? },
                doc! { "$set": { "metadata": bson::to_document(metadata)? } },
                None,
            )
            .await
    }

    /// Aggregates the spending transactions
    pub async fn get_spending_transaction(
        &self,
        transaction_id: &TransactionId,
        idx: u16,
    ) -> Result<Option<BlockRecord>, Error> {
        self.0
            .collection::<BlockRecord>(BlockRecord::COLLECTION)
            .find_one(
                doc! {
                    "inclusion_state": LedgerInclusionState::Included,
                    "block.payload.essence.inputs.transaction_id": bson::to_bson(transaction_id)?,
                    "block.payload.essence.inputs.index": bson::to_bson(&idx)?
                },
                None,
            )
            .await
    }

    /// Finds the block that included a transaction.
    pub async fn get_block_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<BlockRecord>, Error> {
        self.0
            .collection::<BlockRecord>(BlockRecord::COLLECTION)
            .find_one(
                doc! {
                    "inclusion_state": LedgerInclusionState::Included,
                    "block.payload.transaction_id": bson::to_bson(transaction_id)?,
                },
                None,
            )
            .await
    }

    /// Aggregates outputs by transaction ids.
    pub async fn get_output(&self, transaction_id: &TransactionId, idx: u16) -> Result<Option<OutputResult>, Error> {
        Ok(self.0.collection::<BlockRecord>(BlockRecord::COLLECTION).aggregate(
            vec![
                doc! { "$match": { "block.payload.transaction_id": bson::to_bson(transaction_id)? } },
                doc! { "$unwind": { "path": "$block.payload.essence.outputs", "includeArrayIndex": "block.payload.essence.outputs.idx" } },
                doc! { "$match": { "block.payload.essence.outputs.idx": bson::to_bson(&idx)? } },
                doc! { "$project": { "block_id": "$block.id", "metadata": "$metadata", "output": "$block.payload.essence.outputs" } },
            ],
            None,
        )
        .await?
        .try_next()
        .await?
        .map(bson::from_document)
        .transpose()?)
    }

    /// Aggregates the transaction history for an address.
    pub async fn get_transaction_history(
        &self,
        address: &Address,
        page_size: usize,
        page: usize,
        start_milestone: u32,
        end_milestone: u32,
    ) -> Result<impl Stream<Item = Result<Document, Error>>, Error> {
        self.0
        .collection::<BlockRecord>(BlockRecord::COLLECTION)
        .aggregate(vec![
            // Only outputs for this address
            doc! { "$match": {
                "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                "inclusion_state": LedgerInclusionState::Included, 
                "block.payload.essence.outputs.address": bson::to_bson(&address)?
            } },
            doc! { "$set": {
                "block.payload.essence.outputs": {
                    "$filter": {
                        "input": "$block.payload.essence.outputs",
                        "as": "output",
                        "cond": { "$eq": [ "$$output.address", bson::to_bson(&address)? ] }
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
                        "inclusion_state": LedgerInclusionState::Included, 
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
            // Replace the milestone index with the spending transaction's milestone index if there is one
            doc! { "$set": { 
                "milestone_index": { "$cond": [ { "$not": [ "$spending_transaction" ] }, "$milestone_index", "$spending_transaction.0.milestone_index" ] } 
            } },
            doc! { "$sort": { "milestone_index": -1 } },
            doc! { "$skip": (page_size * page) as i64 },
            doc! { "$limit": page_size as i64 },
        ], None)
        .await
    }
}

#[cfg(feature = "analytics")]
impl MongoDb {
    /// Create aggregate statistics of all addresses.
    pub async fn aggregate_addresses(
        &self,
        start_milestone: u32,
        end_milestone: u32,
    ) -> Result<Option<Document>, Error> {
        self.0.collection::<BlockRecord>(BlockRecord::COLLECTION)
        .aggregate(
            vec![
                doc! { "$match": {
                    "inclusion_state": LedgerInclusionState::Included,
                    "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                    "block.payload.kind": "transaction",
                } },
                doc! { "$unwind": { "path": "$block.payload.essence.inputs", "includeArrayIndex": "block.payload.essence.inputs.idx" } },
                doc! { "$lookup": {
                    "from": "stardust_blocks",
                    "let": { "transaction_id": "$block.payload.essence.inputs.transaction_id", "index": "$block.payload.essence.inputs.index" },
                    "pipeline": [
                        { "$match": { 
                            "inclusion_state": LedgerInclusionState::Included, 
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
                doc! { "$set": { "send_address": "$spent_transaction.block.payload.essence.outputs.address" } },
                doc! { "$unwind": { "path": "$block.payload.essence.outputs", "includeArrayIndex": "block.payload.essence.outputs.idx" } },
                doc! { "$set": { "recv_address": "$block.payload.essence.outputs.address" } },
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
        .await?.try_next().await
    }
}
