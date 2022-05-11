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
        stardust::message::{Address, Message, MessageId, Output, TransactionId},
    },
};

/// Chronicle Message record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageRecord {
    /// The message.
    pub message: Message,
    /// The raw bytes of the message.
    #[serde(with = "serde_bytes")]
    pub raw: Vec<u8>,
    /// The message's metadata.
    pub metadata: Option<Metadata>,
}

impl MessageRecord {
    /// The stardust messages collection name.
    pub const COLLECTION: &'static str = "stardust_messages";

    /// Creates a new message record.
    pub fn new(message: Message, raw: Vec<u8>) -> Self {
        Self {
            message,
            raw,
            metadata: None,
        }
    }
}

#[cfg(feature = "inx")]
impl TryFrom<inx::proto::Message> for MessageRecord {
    type Error = inx::Error;

    fn try_from(value: inx::proto::Message) -> Result<Self, Self::Error> {
        let (message, raw_message) = value.try_into()?;
        Ok(Self::new(message.message.into(), raw_message))
    }
}

#[cfg(feature = "inx")]
impl TryFrom<(inx::proto::RawMessage, inx::proto::MessageMetadata)> for MessageRecord {
    type Error = inx::Error;

    fn try_from(
        (raw_message, metadata): (inx::proto::RawMessage, inx::proto::MessageMetadata),
    ) -> Result<Self, Self::Error> {
        let message = bee_message_stardust::Message::try_from(raw_message.clone())?;
        Ok(Self {
            message: message.into(),
            raw: raw_message.data,
            metadata: Some(inx::MessageMetadata::try_from(metadata)?.into()),
        })
    }
}

/// A result received when querying for a single output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputResult {
    /// The id of the message this output came from.
    pub message_id: MessageId,
    /// The metadata of the message this output came from.
    pub metadata: Option<Metadata>,
    /// The output.
    pub output: Output,
}

impl MongoDb {
    /// Get milestone with index.
    pub async fn get_message(&self, message_id: &MessageId) -> Result<Option<MessageRecord>, Error> {
        self.0
            .collection::<MessageRecord>(MessageRecord::COLLECTION)
            .find_one(doc! {"message.id": bson::to_bson(message_id)?}, None)
            .await
    }

    /// Get the children of a message.
    pub async fn get_message_children(
        &self,
        message_id: &MessageId,
        page_size: usize,
        page: usize,
    ) -> Result<impl Stream<Item = Result<MessageRecord, Error>>, Error> {
        self.0
            .collection::<MessageRecord>(MessageRecord::COLLECTION)
            .find(
                doc! {"message.parents": bson::to_bson(message_id)?},
                FindOptions::builder()
                    .skip((page_size * page) as u64)
                    .sort(doc! {"milestone_index": -1})
                    .limit(page_size as i64)
                    .build(),
            )
            .await
    }

    /// Upserts a [`MessageRecord`] to the database.
    pub async fn upsert_message_record(&self, message_record: &MessageRecord) -> Result<UpdateResult, Error> {
        self.0
            .collection::<MessageRecord>(MessageRecord::COLLECTION)
            .update_one(
                doc! { "_id": bson::to_bson(&message_record.message.id)? },
                doc! { "$set": bson::to_document(message_record)? },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
    }

    /// Updates a [`MessageRecord`] with [`Metadata`].
    pub async fn update_message_metadata(
        &self,
        message_id: &MessageId,
        metadata: &Metadata,
    ) -> Result<UpdateResult, Error> {
        self.0
            .collection::<MessageRecord>(MessageRecord::COLLECTION)
            .update_one(
                doc! { "message.id": bson::to_bson(message_id)? },
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
    ) -> Result<Option<MessageRecord>, Error> {
        self.0
            .collection::<MessageRecord>(MessageRecord::COLLECTION)
            .find_one(
                doc! {
                    "inclusion_state": LedgerInclusionState::Included,
                    "message.payload.essence.inputs.transaction_id": bson::to_bson(transaction_id)?,
                    "message.payload.essence.inputs.index": bson::to_bson(&idx)?
                },
                None,
            )
            .await
    }

    /// Finds the message that included a transaction.
    pub async fn get_message_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<MessageRecord>, Error> {
        self.0
            .collection::<MessageRecord>(MessageRecord::COLLECTION)
            .find_one(
                doc! {
                    "inclusion_state": LedgerInclusionState::Included,
                    "message.payload.transaction_id": bson::to_bson(transaction_id)?,
                },
                None,
            )
            .await
    }

    /// Aggregates outputs by transaction ids.
    pub async fn get_output(&self, transaction_id: &TransactionId, idx: u16) -> Result<Option<OutputResult>, Error> {
        Ok(self.0.collection::<MessageRecord>(MessageRecord::COLLECTION).aggregate(
            vec![
                doc! { "$match": { "message.payload.transaction_id": bson::to_bson(transaction_id)? } },
                doc! { "$unwind": { "path": "$message.payload.essence.outputs", "includeArrayIndex": "message.payload.essence.outputs.idx" } },
                doc! { "$match": { "message.payload.essence.outputs.idx": bson::to_bson(&idx)? } },
                doc! { "$project": { "message_id": "$message.id", "metadata": "$metadata", "output": "$message.payload.essence.outputs" } },
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
        .collection::<MessageRecord>(MessageRecord::COLLECTION)
        .aggregate(vec![
            // Only outputs for this address
            doc! { "$match": {
                "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                "inclusion_state": LedgerInclusionState::Included, 
                "message.payload.essence.outputs.address": bson::to_bson(&address)?
            } },
            doc! { "$set": {
                "message.payload.essence.outputs": {
                    "$filter": {
                        "input": "$message.payload.essence.outputs",
                        "as": "output",
                        "cond": { "$eq": [ "$$output.address", bson::to_bson(&address)? ] }
                    }
                }
            } },
            // One result per output
            doc! { "$unwind": { "path": "$message.payload.essence.outputs", "includeArrayIndex": "message.payload.essence.outputs.idx" } },
            // Lookup spending inputs for each output, if they exist
            doc! { "$lookup": {
                "from": "stardust_messages",
                // Keep track of the output id
                "let": { "transaction_id": "$message.payload.transaction_id", "index": "$message.payload.essence.outputs.idx" },
                "pipeline": [
                    // Match using the output's index
                    { "$match": { 
                        "inclusion_state": LedgerInclusionState::Included, 
                        "message.payload.essence.inputs.transaction_id": "$$transaction_id",
                        "message.payload.essence.inputs.index": "$$index"
                    } },
                    { "$set": {
                        "message.payload.essence.inputs": {
                            "$filter": {
                                "input": "$message.payload.essence.inputs",
                                "as": "input",
                                "cond": { "$and": {
                                    "$eq": [ "$$input.transaction_id", "$$transaction_id" ],
                                    "$eq": [ "$$input.index", "$$index" ],
                                } }
                            }
                        }
                    } },
                    // One result per spending input
                    { "$unwind": { "path": "$message.payload.essence.outputs", "includeArrayIndex": "message.payload.essence.outputs.idx" } },
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

#[cfg(feature = "api-analytics")]
impl MongoDb {
    /// Create aggregate statistics of all addresses.
    pub async fn aggregate_addresses(
        &self,
        start_milestone: u32,
        end_milestone: u32,
    ) -> Result<Option<Document>, Error> {
        self.0.collection::<MessageRecord>(MessageRecord::COLLECTION)
        .aggregate(
            vec![
                doc! { "$match": {
                    "inclusion_state": LedgerInclusionState::Included,
                    "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                    "message.payload.kind": "transaction",
                } },
                doc! { "$unwind": { "path": "$message.payload.essence.inputs", "includeArrayIndex": "message.payload.essence.inputs.idx" } },
                doc! { "$lookup": {
                    "from": "stardust_messages",
                    "let": { "transaction_id": "$message.payload.essence.inputs.transaction_id", "index": "$message.payload.essence.inputs.index" },
                    "pipeline": [
                        { "$match": { 
                            "inclusion_state": LedgerInclusionState::Included, 
                            "message.payload.transaction_id": "$$transaction_id",
                        } },
                        { "$set": {
                            "message.payload.essence.outputs": {
                                "$arrayElemAt": [
                                    "$message.payload.essence.outputs",
                                    "$$index"
                                ]
                            }
                        } },
                    ],
                    "as": "spent_transaction"
                } },
                doc! { "$set": { "send_address": "$spent_transaction.message.payload.essence.outputs.address" } },
                doc! { "$unwind": { "path": "$message.payload.essence.outputs", "includeArrayIndex": "message.payload.essence.outputs.idx" } },
                doc! { "$set": { "recv_address": "$message.payload.essence.outputs.address" } },
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
