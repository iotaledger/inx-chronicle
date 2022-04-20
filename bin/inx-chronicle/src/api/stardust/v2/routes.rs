// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{
    extract::{Extension, Path},
    routing::*,
    Router,
};
use chronicle::{
    bson::{BsonExt, DocExt, U64},
    db::{
        model::{
            inclusion_state::LedgerInclusionState,
            stardust::{message::MessageRecord, milestone::MilestoneRecord},
        },
        MongoDatabase,
    },
    stardust::payload::transaction::TransactionPayload,
};
use futures::TryStreamExt;
use mongodb::{bson::doc, options::FindOptions};

use super::responses::*;
use crate::api::{
    error::ApiError,
    extractors::{Expanded, Pagination, TimeRange},
    responses::Record,
    stardust::{end_milestone, start_milestone},
    ApiResult,
};

pub fn routes() -> Router {
    Router::new()
        .nest(
            "/messages",
            Router::new()
                .route("/:message_id", get(message))
                .route("/:message_id/raw", get(message_raw))
                .route("/:message_id/metadata", get(message_metadata))
                .route("/:message_id/children", get(message_children)),
        )
        .nest("/outputs", Router::new().route("/:transaction_id/:idx", get(output)))
        .nest(
            "/transactions",
            Router::new()
                .route("/:message_id", get(transaction_for_message))
                .route("/included-message/:transaction_id", get(transaction_included_message)),
        )
        .route("/milestones/:index", get(milestone))
        .nest("/analytics", Router::new().route("/addresses", get(address_analytics)))
}

async fn message(database: Extension<MongoDatabase>, Path(message_id): Path<String>) -> ApiResult<MessageResponse> {
    let mut rec = database
        .collection::<MessageRecord>()
        .find_one(doc! {"message_id": &message_id}, None)
        .await?
        .ok_or(ApiError::NoResults)?;
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

async fn message_raw(database: Extension<MongoDatabase>, Path(message_id): Path<String>) -> ApiResult<Vec<u8>> {
    let mut rec = database
        .collection::<MessageRecord>()
        .find_one(doc! {"message_id": &message_id}, None)
        .await?
        .ok_or(ApiError::NoResults)?;
    let mut message = rec.take_document("message")?;
    Ok(message.take_bytes("raw")?)
}

async fn message_metadata(
    database: Extension<MongoDatabase>,
    Path(message_id): Path<String>,
) -> ApiResult<MessageMetadataResponse> {
    let mut rec = database
        .collection::<MessageRecord>()
        .find_one(doc! {"message_id": &message_id}, None)
        .await?
        .ok_or(ApiError::NoResults)?;
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
    database: Extension<MongoDatabase>,
    Path(message_id): Path<String>,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
) -> ApiResult<MessageChildrenResponse> {
    let messages = database
        .collection::<MessageRecord>()
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
        message_id,
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
            .collect::<Result<_, ApiError>>()?,
    })
}

async fn output(
    database: Extension<MongoDatabase>,
    Path((transaction_id, idx)): Path<(String, u16)>,
) -> ApiResult<OutputResponse> {
    let mut output = database
        .collection::<MessageRecord>()
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
        .await?.ok_or(ApiError::NoResults)?;

    let spending_transaction = database
        .collection::<MessageRecord>()
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

async fn transaction_for_message(
    database: Extension<MongoDatabase>,
    Path(message_id): Path<String>,
) -> ApiResult<TransactionResponse> {
    let mut rec = database
        .collection::<MessageRecord>()
        .find_one(doc! {"message_id": &message_id}, None)
        .await?
        .ok_or(ApiError::NoResults)?;
    let mut essence = rec.take_path("message.payload.data.essence.data")?.to_document()?;

    Ok(TransactionResponse {
        message_id,
        milestone_index: rec.take_bson("milestone_index").ok().map(|b| b.as_u32()).transpose()?,
        outputs: essence.take_array("outputs")?.into_iter().map(Into::into).collect(),
        inputs: essence.take_array("inputs")?.into_iter().map(Into::into).collect(),
    })
}

async fn transaction_included_message(
    database: Extension<MongoDatabase>,
    Path(transaction_id): Path<String>,
) -> ApiResult<MessageResponse> {
    let mut rec = database
        .collection::<MessageRecord>()
        .find_one(
            doc! {
                "inclusion_state": LedgerInclusionState::Included as u8 as i32,
                "message.payload.transaction_id": &transaction_id.to_string(),
            },
            None,
        )
        .await?
        .ok_or(ApiError::NoResults)?;
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

async fn milestone(database: Extension<MongoDatabase>, Path(index): Path<u32>) -> ApiResult<MilestoneResponse> {
    database
        .collection::<MilestoneRecord>()
        .find_one(doc! {"milestone_index": &index}, None)
        .await?
        .ok_or(ApiError::NoResults)
        .and_then(|rec| {
            Ok(MilestoneResponse {
                milestone_index: index,
                message_id: rec.get_as_string("message_id")?,
                timestamp: rec.get_as_u32("milestone_timestamp")?,
            })
        })
}

async fn address_analytics(
    database: Extension<MongoDatabase>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<AddressAnalyticsResponse> {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let res = database
        .collection::<MessageRecord>()
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
        .await?.try_next().await?.ok_or(ApiError::NoResults)?;

    Ok(AddressAnalyticsResponse {
        total_addresses: res.get_as_u64("total_addresses")?,
        recv_addresses: res.get_as_u64("recv_addresses")?,
        send_addresses: res.get_as_u64("send_addresses")?,
    })
}
