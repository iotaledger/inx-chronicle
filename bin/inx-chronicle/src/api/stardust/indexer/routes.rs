// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{routing::get, Extension, Router};
use chronicle::{
    db::{
        bson::{BsonExt, DocExt},
        model::{inclusion_state::LedgerInclusionState, stardust::message::MessageRecord},
        MongoDb,
    },
    stardust::{output::OutputId, payload::transaction::TransactionId},
};
use futures::TryStreamExt;
use mongodb::{bson::doc, options::FindOptions};

use super::{
    extractors::{MessagesQuery, OutputsQuery},
    responses::{MessagesForQueryResponse, OutputsForQueryResponse},
};
use crate::api::{
    extractors::{Expanded, Pagination, TimeRange},
    responses::Record,
    stardust::{end_milestone, start_milestone},
    ApiError, ApiResult,
};

pub fn routes() -> Router {
    Router::new()
        .nest("/messages", Router::new().route("/", get(messages_query)))
        .nest("/outputs", Router::new().route("/", get(outputs_query)))
}

async fn messages_query(
    database: Extension<MongoDb>,
    query: MessagesQuery,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<MessagesForQueryResponse> {
    let MessagesQuery { tag, included: _ } = &query;
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let mut query_doc = doc! { "milestone_index": { "$gt": start_milestone, "$lt": end_milestone } };
    if let Some(tag) = tag.as_ref() {
        query_doc.insert("message.payload.tag", tag);
    }

    let messages = database
        .doc_collection::<MessageRecord>()
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
            .collect::<Result<_, ApiError>>()?,
    })
}

async fn outputs_query(
    database: Extension<MongoDb>,
    query: OutputsQuery,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<OutputsForQueryResponse> {
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
            .insert("inclusion_state", LedgerInclusionState::Included);
    }

    let outputs = database
        .doc_collection::<MessageRecord>()
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
            .collect::<Result<_, ApiError>>()?,
    })
}
