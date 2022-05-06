// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::{doc, Document},
    error::Error,
    options::{FindOptions, UpdateOptions},
    results::UpdateResult,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{bson, model::inclusion_state::LedgerInclusionState, MongoDb},
    stardust::{payload::transaction::TransactionId, semantic::ConflictReason, Message, MessageId},
};

/// Chronicle Message record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageRecord {
    /// The message ID.
    pub message_id: MessageId,
    /// The message.
    pub message: Message,
    /// The raw bytes of the message.
    #[serde(with = "serde_bytes")]
    pub raw: Vec<u8>,
    /// The message's metadata.
    pub metadata: Option<MessageMetadata>,
}

impl MessageRecord {
    /// The stardust messages collection name.
    pub const COLLECTION: &'static str = "stardust_messages";

    /// Creates a new message record.
    pub fn new(message: Message, raw: Vec<u8>) -> Self {
        Self {
            message_id: message.id(),
            message,
            raw,
            metadata: None,
        }
    }
}

impl TryFrom<inx::proto::Message> for MessageRecord {
    type Error = inx::Error;

    fn try_from(value: inx::proto::Message) -> Result<Self, Self::Error> {
        let (message, raw_message) = value.try_into()?;
        Ok(Self::new(message.message, raw_message))
    }
}

impl TryFrom<(inx::proto::RawMessage, inx::proto::MessageMetadata)> for MessageRecord {
    type Error = inx::Error;

    fn try_from(
        (raw_message, metadata): (inx::proto::RawMessage, inx::proto::MessageMetadata),
    ) -> Result<Self, Self::Error> {
        let message = Message::try_from(raw_message.clone())?;
        Ok(Self {
            message_id: message.id(),
            message,
            raw: raw_message.data,
            metadata: Some(inx::MessageMetadata::try_from(metadata)?.into()),
        })
    }
}

/// Message metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Status of the solidification process.
    pub is_solid: bool,
    /// Indicates that the message should be promoted.
    pub should_promote: bool,
    /// Indicates that the message should be reattached.
    pub should_reattach: bool,
    /// The milestone index referencing the message.
    pub referenced_by_milestone_index: u32,
    /// The corresponding milestone index.
    pub milestone_index: u32,
    /// The inclusion state of the message.
    pub inclusion_state: LedgerInclusionState,
    /// If the ledger inclusion state is conflicting, the reason for the conflict.
    pub conflict_reason: Option<ConflictReason>,
}

impl MongoDb {
    /// Get milestone with index.
    pub async fn get_message(&self, message_id: &MessageId) -> Result<Option<Document>, Error> {
        self.0
            .collection::<Document>(MessageRecord::COLLECTION)
            .find_one(doc! {"message_id": message_id.to_string()}, None)
            .await
    }

    /// Get the children of a message.
    pub async fn get_message_children(
        &self,
        message_id: &MessageId,
        page_size: usize,
        page: usize,
    ) -> Result<impl Stream<Item = Result<Document, Error>>, Error> {
        self.0
            .collection::<Document>(MessageRecord::COLLECTION)
            .find(
                doc! {"message.parents": message_id.to_string()},
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
        let doc = bson::to_document(message_record)?;
        self.0
            .collection::<Document>(MessageRecord::COLLECTION)
            .update_one(
                doc! { "_id": message_record.message_id.to_string() },
                doc! { "$set": doc },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
    }

    /// Updates a [`MessageRecord`] with [`MessageMetadata`].
    pub async fn update_message_metadata(
        &self,
        message_id: &MessageId,
        metadata: &MessageMetadata,
    ) -> Result<UpdateResult, Error> {
        self.0
            .collection::<Document>(MessageRecord::COLLECTION)
            .update_one(
                doc! { "message_id": message_id.to_string() },
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
    ) -> Result<Option<Document>, Error> {
        self.0
            .collection::<Document>(MessageRecord::COLLECTION)
            .find_one(
                doc! {
                    "inclusion_state": LedgerInclusionState::Included,
                    "message.payload.data.essence.data.inputs.transaction_id": transaction_id.to_string(),
                    "message.payload.data.essence.data.inputs.index": idx as i64
                },
                None,
            )
            .await
    }

    /// Finds the message that included a transaction.
    pub async fn get_message_for_transaction(&self, transaction_id: &str) -> Result<Option<Document>, Error> {
        self.0
            .collection::<Document>(MessageRecord::COLLECTION)
            .find_one(
                doc! {
                    "inclusion_state": LedgerInclusionState::Included,
                    "message.payload.transaction_id": transaction_id,
                },
                None,
            )
            .await
    }

    /// Aggregates outputs by transaction ids.
    pub async fn get_output(&self, transaction_id: &TransactionId, idx: u16) -> Result<Option<Document>, Error> {
        self.0.collection::<Document>(MessageRecord::COLLECTION).aggregate(
            vec![
                doc! { "$match": { "message.payload.transaction_id": transaction_id.to_string() } },
                doc! { "$unwind": { "path": "$message.payload.data.essence.data.outputs", "includeArrayIndex": "message.payload.data.essence.data.outputs.idx" } },
                doc! { "$match": { "message.payload.data.essence.data.outputs.idx": idx as i64 } },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Aggregates the transaction history for an address.
    pub async fn get_transaction_history(
        &self,
        address: &str,
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
                "message.payload.data.essence.data.outputs.address.data": &address 
            } },
            doc! { "$set": {
                "message.payload.data.essence.data.outputs": {
                    "$filter": {
                        "input": "$message.payload.data.essence.data.outputs",
                        "as": "output",
                        "cond": { "$eq": [ "$$output.address.data", &address ] }
                    }
                }
            } },
            // One result per output
            doc! { "$unwind": { "path": "$message.payload.data.essence.data.outputs", "includeArrayIndex": "message.payload.data.essence.data.outputs.idx" } },
            // Lookup spending inputs for each output, if they exist
            doc! { "$lookup": {
                "from": "stardust_messages",
                // Keep track of the output id
                "let": { "transaction_id": "$message.payload.transaction_id", "index": "$message.payload.data.essence.data.outputs.idx" },
                "pipeline": [
                    // Match using the output's index
                    { "$match": { 
                        "inclusion_state": LedgerInclusionState::Included, 
                        "message.payload.data.essence.data.inputs.transaction_id": "$$transaction_id",
                        "message.payload.data.essence.data.inputs.index": "$$index"
                    } },
                    { "$set": {
                        "message.payload.data.essence.data.inputs": {
                            "$filter": {
                                "input": "$message.payload.data.essence.data.inputs",
                                "as": "input",
                                "cond": { "$and": {
                                    "$eq": [ "$$input.transaction_id", "$$transaction_id" ],
                                    "$eq": [ "$$input.index", "$$index" ],
                                } }
                            }
                        }
                    } },
                    // One result per spending input
                    { "$unwind": { "path": "$message.payload.data.essence.data.outputs", "includeArrayIndex": "message.payload.data.essence.data.outputs.idx" } },
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
            doc! { "$sort": { "milestone_index": -1i32 } },
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
        use crate::stardust::payload::TransactionPayload;

        self.0.collection::<Document>(MessageRecord::COLLECTION)
        .aggregate(
            vec![
                doc! { "$match": {
                    "inclusion_state": LedgerInclusionState::Included,
                    "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                    "message.payload.data.kind": TransactionPayload::KIND as i32,
                } },
                doc! { "$unwind": { "path": "$message.payload.data.essence.data.inputs", "includeArrayIndex": "message.payload.data.essence.data.inputs.idx" } },
                doc! { "$lookup": {
                    "from": "stardust_messages",
                    "let": { "transaction_id": "$message.payload.data.essence.data.inputs.transaction_id", "index": "$message.payload.data.essence.data.inputs.index" },
                    "pipeline": [
                        { "$match": { 
                            "inclusion_state": LedgerInclusionState::Included, 
                            "message.payload.transaction_id": "$$transaction_id",
                        } },
                        { "$set": {
                            "message.payload.data.essence.data.outputs": {
                                "$arrayElemAt": [
                                    "$message.payload.data.essence.data.outputs",
                                    "$$index"
                                ]
                            }
                        } },
                    ],
                    "as": "spent_transaction"
                } },
                doc! { "$set": { "send_address": "$spent_transaction.message.payload.data.essence.data.outputs.address.data" } },
                doc! { "$unwind": { "path": "$message.payload.data.essence.data.outputs", "includeArrayIndex": "message.payload.data.essence.data.outputs.idx" } },
                doc! { "$set": { "recv_address": "$message.payload.data.essence.data.outputs.address.data" } },
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
                    "total_addresses": { "$arrayElemAt": ["$total.addresses", 0u32] },
                    "recv_addresses": { "$arrayElemAt": ["$recv.addresses", 0u32] },
                    "send_addresses": { "$arrayElemAt": ["$send.addresses", 0u32] },
                } },
            ],
            None,
        )
        .await?.try_next().await
    }
}

impl From<inx::MessageMetadata> for MessageMetadata {
    fn from(metadata: inx::MessageMetadata) -> Self {
        Self {
            is_solid: metadata.is_solid,
            should_promote: metadata.should_promote,
            should_reattach: metadata.should_reattach,
            referenced_by_milestone_index: metadata.referenced_by_milestone_index,
            milestone_index: metadata.milestone_index,
            inclusion_state: match metadata.ledger_inclusion_state {
                inx::LedgerInclusionState::Included => LedgerInclusionState::Included,
                inx::LedgerInclusionState::NoTransaction => LedgerInclusionState::NoTransaction,
                inx::LedgerInclusionState::Conflicting => LedgerInclusionState::Conflicting,
            },
            conflict_reason: match metadata.conflict_reason {
                inx::ConflictReason::None => None,
                inx::ConflictReason::InputAlreadySpent => Some(ConflictReason::InputUtxoAlreadySpent),
                inx::ConflictReason::InputAlreadySpentInThisMilestone => {
                    Some(ConflictReason::InputUtxoAlreadySpentInThisMilestone)
                }
                inx::ConflictReason::InputNotFound => Some(ConflictReason::InputUtxoNotFound),
                inx::ConflictReason::InputOutputSumMismatch => todo!(),
                inx::ConflictReason::InvalidSignature => Some(ConflictReason::InvalidSignature),
                inx::ConflictReason::InvalidNetworkId => todo!(),
                inx::ConflictReason::SemanticValidationFailed => Some(ConflictReason::SemanticValidationFailed),
            },
        }
    }
}
