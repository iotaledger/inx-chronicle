// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{routing::get, Extension, Router};
use chronicle::{
    bson::DocExt,
    db::{
        model::{inclusion_state::LedgerInclusionState, stardust::message::MessageRecord},
        MongoDatabase,
    },
    stardust::payload::TransactionPayload,
};
use futures::TryStreamExt;
use mongodb::bson::doc;

use super::responses::AddressAnalyticsResponse;
use crate::api::{
    extractors::TimeRange,
    stardust::{end_milestone, start_milestone},
    ApiError, ApiResult,
};

pub fn routes() -> Router {
    Router::new().nest("/analytics", Router::new().route("/addresses", get(address_analytics)))
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
        .doc_collection::<MessageRecord>()
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
