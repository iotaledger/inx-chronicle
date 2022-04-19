// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Extension, Path},
    routing::*,
    Router,
};
use chronicle::{
    bson::U64,
    chrysalis::{
        payload::transaction::TransactionPayload,
        prelude::{OutputId, TransactionId},
    },
    db::model::inclusion_state::LedgerInclusionState,
    BsonExt, DocExt,
};
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, DateTime, Document},
    options::FindOptions,
    Database,
};
use time::OffsetDateTime;

use super::{
    extractors::{MessagesQuery, OutputsQuery},
    responses::*,
};
use crate::api::{
    error::APIError,
    extractors::{Expanded, Pagination, TimeRange},
    APIResult,
};

pub fn routes() -> Router {
    Router::new()
        .nest(
            "/messages",
            Router::new()
                .route("/", get(messages_query))
                .route("/:message_id", get(message))
                .route("/:message_id/raw", get(message_raw))
                .route("/:message_id/metadata", get(message_metadata))
                .route("/:message_id/children", get(message_children)),
        )
        .nest(
            "/outputs",
            Router::new()
                .route("/", get(outputs_query))
                .route("/:transaction_id/:idx", get(output)),
        )
        .nest(
            "/transactions",
            Router::new()
                .route("/:message_id", get(transaction_for_message))
                .route("/history/:address", get(transaction_history))
                .route("/included-message/:transaction_id", get(transaction_included_message)),
        )
        .route("/milestones/:index", get(milestone))
        .nest("/analytics", Router::new().route("/addresses", get(address_analytics)))
}

async fn message(database: Extension<Database>, Path(message_id): Path<String>) -> APIResult<MessageResponse> {
    let mut rec = database
        .collection::<Document>("stardust_messages")
        .find_one(doc! {"message_id": &message_id}, None)
        .await?
        .ok_or(APIError::NoResults)?;
    let mut message = rec.take_document("message")?;
    Ok(MessageResponse {
        protocol_version: message.get_as_u8("protocol_version")?,
        parents: message
            .take_array("parents")?
            .iter()
            .map(|m| m.as_string())
            .collect::<Result<_, _>>()?,
        payload: message.take_bson("payload").ok().map(Into::into),
        nonce: message.convert_document::<U64, _>("nonce")?.into(),
    })
}

async fn message_raw(database: Extension<Database>, Path(message_id): Path<String>) -> APIResult<Vec<u8>> {
    let mut rec = database
        .collection::<Document>("stardust_messages")
        .find_one(doc! {"message_id": &message_id}, None)
        .await?
        .ok_or(APIError::NoResults)?;
    let mut message = rec.take_document("message")?;
    Ok(message.take_bytes("raw")?)
}

async fn message_metadata(
    database: Extension<Database>,
    Path(message_id): Path<String>,
) -> APIResult<MessageMetadataResponse> {
    let mut rec = database
        .collection::<Document>("stardust_messages")
        .find_one(doc! {"message_id": &message_id}, None)
        .await?
        .ok_or(APIError::NoResults)?;
    let mut message = rec.take_document("message")?;
    let inclusion_state = rec.get("inclusion_state").map(|b| b.as_u8()).transpose()?;
    let milestone_index = rec.get("milestone_index").map(|b| b.as_u32()).transpose()?;
    let conflict_reason = rec.get("conflict_reason").map(|b| b.as_u8()).transpose()?;

    Ok(MessageMetadataResponse {
        message_id: rec.get_as_string("message_id")?,
        parent_message_ids: message
            .take_array("parents")?
            .iter()
            .map(|id| id.as_string())
            .collect::<Result<_, _>>()?,
        is_solid: inclusion_state.is_some(),
        referenced_by_milestone_index: inclusion_state.and(milestone_index),
        milestone_index: inclusion_state.and(milestone_index),
        should_promote: Some(inclusion_state.is_none()),
        should_reattach: Some(inclusion_state.is_none()),
        ledger_inclusion_state: inclusion_state.map(TryInto::try_into).transpose()?,
        conflict_reason,
    })
}

async fn message_children(
    database: Extension<Database>,
    Path(message_id): Path<String>,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
) -> APIResult<MessageChildrenResponse> {
    let messages = database
        .collection::<Document>("stardust_messages")
        .find(
            doc! {"message.parents": &message_id},
            FindOptions::builder()
                .skip((page_size * page) as u64)
                .sort(doc! {"milestone_index": -1})
                .limit(page_size as i64)
                .build(),
        )
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(MessageChildrenResponse {
        message_id: message_id,
        max_results: page_size,
        count: messages.len(),
        children_message_ids: messages
            .into_iter()
            .map(|mut rec| {
                let message = rec.take_document("message")?;
                if expanded {
                    let inclusion_state = rec.get("inclusion_state").map(|b| b.as_u8()).transpose()?;
                    let milestone_index = rec.get("milestone_index").map(|b| b.as_u32()).transpose()?;
                    Ok(Record {
                        id: message.get_as_string("message_id")?,
                        inclusion_state: inclusion_state.map(TryInto::try_into).transpose()?,
                        milestone_index,
                    }
                    .into())
                } else {
                    Ok(message.get_as_string("message_id")?.into())
                }
            })
            .collect::<Result<_, APIError>>()?,
    })
}

async fn start_milestone(database: &Database, start_timestamp: OffsetDateTime) -> APIResult<u32> {
    database
        .collection::<Document>("stardust_milestones")
        .find(
            doc! {"milestone_timestamp": { "$gte": DateTime::from_millis(start_timestamp.unix_timestamp() * 1000) }},
            FindOptions::builder()
                .sort(doc! {"milestone_index": 1})
                .limit(1)
                .build(),
        )
        .await?
        .try_next()
        .await?
        .map(|d| d.get_as_u32("milestone_index"))
        .transpose()?
        .ok_or(APIError::NotFound)
}

async fn end_milestone(database: &Database, end_timestamp: OffsetDateTime) -> APIResult<u32> {
    database
        .collection::<Document>("stardust_milestones")
        .find(
            doc! {"milestone_timestamp": { "$lte": DateTime::from_millis(end_timestamp.unix_timestamp() * 1000) }},
            FindOptions::builder()
                .sort(doc! {"milestone_index": -1})
                .limit(1)
                .build(),
        )
        .await?
        .try_next()
        .await?
        .map(|d| d.get_as_u32("milestone_index"))
        .transpose()?
        .ok_or(APIError::NotFound)
}

async fn messages_query(
    database: Extension<Database>,
    query: MessagesQuery,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> APIResult<MessagesForQueryResponse> {
    let MessagesQuery { tag, included: _ } = &query;
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let mut query_doc = doc! { "milestone_index": { "$gt": start_milestone, "$lt": end_milestone } };
    if let Some(tag) = tag.as_ref() {
        query_doc.insert("message.payload.tag", tag);
    }

    let messages = database
        .collection::<Document>("stardust_messages")
        .find(
            query_doc,
            FindOptions::builder()
                .skip((page_size * page) as u64)
                .sort(doc! {"milestone_index": -1})
                .limit(page_size as i64)
                .build(),
        )
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(MessagesForQueryResponse {
        query,
        max_results: page_size,
        count: messages.len(),
        message_ids: messages
            .into_iter()
            .map(|mut rec| {
                let message = rec.take_document("message")?;
                if expanded {
                    let inclusion_state = rec.get("inclusion_state").map(|b| b.as_u8()).transpose()?;
                    let milestone_index = rec.get("milestone_index").map(|b| b.as_u32()).transpose()?;
                    Ok(Record {
                        id: message.get_as_string("message_id")?,
                        inclusion_state: inclusion_state.map(TryInto::try_into).transpose()?,
                        milestone_index,
                    }
                    .into())
                } else {
                    Ok(message.get_as_string("message_id")?.into())
                }
            })
            .collect::<Result<_, APIError>>()?,
    })
}

async fn output(
    database: Extension<Database>,
    Path((transaction_id, idx)): Path<(String, u16)>,
) -> APIResult<OutputResponse> {
    let mut output = database
        .collection::<Document>("stardust_messages")
        .aggregate(
            vec![
                doc! { "$match": { "message.payload.transaction_id": &transaction_id.to_string() } },
                doc! { "$unwind": { "path": "$message.payload.data.essence.data.outputs", "includeArrayIndex": "message.payload.data.essence.data.outputs.idx" } },
                doc! { "$match": { "message.payload.data.essence.data.outputs.idx": idx as i64 } },
            ],
            None,
        )
        .await?
        .try_next()
        .await?.ok_or(APIError::NoResults)?;

    let spending_transaction = database
        .collection::<Document>("stardust_messages")
        .find_one(
            doc! {
                "inclusion_state": LedgerInclusionState::Included as u8 as i32,
                "message.payload.data.essence.data.inputs.transaction_id": &transaction_id.to_string(),
                "message.payload.data.essence.data.inputs.index": idx as i64
            },
            None,
        )
        .await?;

    Ok(OutputResponse {
        message_id: output.get_str("message_id")?.to_owned(),
        transaction_id: transaction_id.to_string(),
        output_index: idx,
        spending_transaction: spending_transaction
            .map(|mut d| d.take_bson("message"))
            .transpose()?
            .map(Into::into),
        output: output.take_path("message.payload.data.essence.data.outputs")?.into(),
    })
}

async fn outputs_query(
    database: Extension<Database>,
    query: OutputsQuery,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> APIResult<OutputsForQueryResponse> {
    let OutputsQuery {
        address,
        included,
        requires_dust_return: _,
        sender: _,
        tag: _,
    } = &query;
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let mut query_doc =
        vec![doc! { "$match": { "milestone_index": { "$gt": start_milestone, "$lt": end_milestone } } }];

    if let Some(address) = address.as_ref() {
        query_doc.extend([
            doc! { "$match": { "message.payload.data.essence.data.outputs.address.data": address } },
            doc! { "$set": {
                "message.payload.data.essence.data.outputs": {
                    "$filter": {
                        "input": "$message.payload.data.essence.data.outputs",
                        "as": "output",
                        "cond": { "$eq": [ "$$output.address.data", address ] }
                    }
                }
            } },
        ]);
    }
    query_doc.extend([
        doc! { "$unwind": { "path": "$message.payload.data.essence.data.outputs", "includeArrayIndex": "message.payload.data.essence.data.outputs.idx" } },
        doc! { "$sort": { "milestone_index": -1 } },
        doc! { "$skip": (page_size * page) as i64 },
        doc! { "$limit": page_size as i64 },
    ]);
    if *included {
        query_doc[0]
            .get_document_mut("$match")?
            .insert("inclusion_state", LedgerInclusionState::Included as u8 as i32);
    }

    let outputs = database
        .collection::<Document>("stardust_messages")
        .aggregate(query_doc, None)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(OutputsForQueryResponse {
        query,
        max_results: page_size,
        count: outputs.len(),
        output_ids: outputs
            .into_iter()
            .map(|mut rec| {
                let mut payload = rec.take_path("message.payload.data")?.to_document()?;
                let transaction_id = TransactionId::from_str(payload.get_str("transaction_id")?)?;
                let idx = payload
                    .take_path("essence.data.outputs")?
                    .to_document()?
                    .get_as_u16("idx")?;
                let output_id = OutputId::new(transaction_id, idx)?;
                Ok(if expanded {
                    let inclusion_state = rec
                        .get_as_u8("inclusion_state")
                        .ok()
                        .map(LedgerInclusionState::try_from)
                        .transpose()?;
                    let milestone_index = rec.get_i32("milestone_index").ok().map(|m| m as u32);
                    Record {
                        id: output_id.to_string(),
                        inclusion_state,
                        milestone_index,
                    }
                    .into()
                } else {
                    output_id.to_string().into()
                })
            })
            .collect::<Result<_, APIError>>()?,
    })
}

async fn transaction_history(
    database: Extension<Database>,
    Path(address): Path<String>,
    Pagination { page_size, page }: Pagination,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> APIResult<TransactionHistoryResponse> {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let records = database
        .collection::<Document>("stardust_messages")
        .aggregate(vec![
            // Only outputs for this address
            doc! { "$match": {
                "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                "inclusion_state": LedgerInclusionState::Included as u8 as i32, 
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
                        "inclusion_state": LedgerInclusionState::Included as u8 as i32, 
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
            doc! { "$sort": { "milestone_index": -1 } },
            doc! { "$skip": (page_size * page) as i64 },
            doc! { "$limit": page_size as i64 },
        ], None)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let transactions = records
        .into_iter()
        .map(|mut rec| {
            let mut payload = rec.take_path("message.payload.data")?.to_document()?;
            let spending_transaction = rec.take_document("spending_transaction").ok();
            let output = payload.take_path("essence.data.outputs")?.to_document()?;
            Ok(Transfer {
                transaction_id: payload.get_as_string("transaction_id")?,
                output_index: output.get_as_u16("idx")?,
                is_spending: spending_transaction.is_some(),
                inclusion_state: rec
                    .get_as_u8("inclusion_state")
                    .ok()
                    .map(LedgerInclusionState::try_from)
                    .transpose()?,
                message_id: rec.get_as_string("message_id")?,
                amount: output.get_as_u64("amount")?,
            })
        })
        .collect::<Result<_, APIError>>()?;

    Ok(TransactionHistoryResponse { transactions, address })
}

async fn transaction_for_message(
    database: Extension<Database>,
    Path(message_id): Path<String>,
) -> APIResult<TransactionResponse> {
    let mut rec = database
        .collection::<Document>("stardust_messages")
        .find_one(doc! {"message_id": &message_id}, None)
        .await?
        .ok_or(APIError::NoResults)?;
    let mut essence = rec.take_path("message.payload.data.essence.data")?.to_document()?;

    Ok(TransactionResponse {
        message_id,
        milestone_index: rec.take_bson("milestone_index").ok().map(|b| b.as_u32()).transpose()?,
        outputs: essence.take_array("outputs")?.into_iter().map(Into::into).collect(),
        inputs: essence.take_array("inputs")?.into_iter().map(Into::into).collect(),
    })
}

async fn transaction_included_message(
    database: Extension<Database>,
    Path(transaction_id): Path<String>,
) -> APIResult<MessageResponse> {
    let mut rec = database
        .collection::<Document>("stardust_messages")
        .find_one(
            doc! {
                "inclusion_state": LedgerInclusionState::Included as u8 as i32,
                "message.payload.transaction_id": &transaction_id.to_string(),
            },
            None,
        )
        .await?
        .ok_or(APIError::NoResults)?;
    let mut message = rec.take_document("message")?;

    Ok(MessageResponse {
        protocol_version: message.get_as_u8("protocol_version")?,
        parents: message
            .take_array("parents")?
            .iter()
            .map(|m| m.as_string())
            .collect::<Result<_, _>>()?,
        payload: message.take_bson("payload").ok().map(Into::into),
        nonce: message.convert_document::<U64, _>("nonce")?.into(),
    })
}

async fn milestone(database: Extension<Database>, Path(index): Path<u32>) -> APIResult<MilestoneResponse> {
    database
        .collection::<Document>("stardust_milestones")
        .find_one(doc! {"milestone_index": &index}, None)
        .await?
        .ok_or(APIError::NoResults)
        .and_then(|rec| {
            Ok(MilestoneResponse {
                milestone_index: index,
                message_id: rec.get_as_string("message_id")?,
                timestamp: rec.get_as_u32("milestone_timestamp")?,
            })
        })
}

async fn address_analytics(
    database: Extension<Database>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> APIResult<AddressAnalyticsResponse> {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let res = database
        .collection::<Document>("stardust_messages")
        .aggregate(
            vec![
                doc! { "$match": {
                    "inclusion_state": LedgerInclusionState::Included as u8 as i32,
                    "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                    "message.payload.data.kind": TransactionPayload::KIND as i32,
                } },
                doc! { "$unwind": { "path": "$message.payload.data.essence.data.inputs", "includeArrayIndex": "message.payload.data.essence.data.inputs.idx" } },
                doc! { "$lookup": {
                    "from": "stardust_messages",
                    "let": { "transaction_id": "$message.payload.data.essence.data.inputs.transaction_id", "index": "$message.payload.data.essence.data.inputs.index" },
                    "pipeline": [
                        { "$match": { 
                            "inclusion_state": LedgerInclusionState::Included as u8 as i32, 
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
                    "total_addresses": { "$arrayElemAt": ["$total.addresses", 0] },
                    "recv_addresses": { "$arrayElemAt": ["$recv.addresses", 0] },
                    "send_addresses": { "$arrayElemAt": ["$send.addresses", 0] },
                } },
            ],
            None,
        )
        .await?.try_next().await?.ok_or(APIError::NoResults)?;

    Ok(AddressAnalyticsResponse {
        total_addresses: res.get_as_u64("total_addresses")?,
        recv_addresses: res.get_as_u64("recv_addresses")?,
        send_addresses: res.get_as_u64("send_addresses")?,
    })
}
