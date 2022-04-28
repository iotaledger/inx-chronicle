// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    bson::{BsonExt, DocExt},
    db::{
        model::{inclusion_state::LedgerInclusionState, stardust::message::MessageRecord},
        MongoDb,
    },
};
use futures::TryStreamExt;
use mongodb::bson::doc;

use super::responses::TransactionHistoryResponse;
use crate::api::{
    extractors::{Pagination, TimeRange},
    responses::Transfer,
    stardust::{end_milestone, start_milestone},
    ApiError, ApiResult,
};

pub fn routes() -> Router {
    Router::new().nest(
        "/transactions",
        Router::new().route("/history/:address", get(transaction_history)),
    )
}

async fn transaction_history(
    database: Extension<MongoDb>,
    Path(address): Path<String>,
    Pagination { page_size, page }: Pagination,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<TransactionHistoryResponse> {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let records = database
        .collection::<MessageRecord>()
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
        .collect::<Result<_, ApiError>>()?;

    Ok(TransactionHistoryResponse { transactions, address })
}
