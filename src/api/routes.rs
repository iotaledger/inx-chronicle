// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{
        Extension,
        Path,
    },
    handler::Handler,
    http::Method,
    routing::*,
    Json,
    Router,
};
use futures::TryStreamExt;
use mongodb::{
    bson::{
        doc,
        DateTime,
        Document,
    },
    options::FindOptions,
    Database,
};
use time::OffsetDateTime;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{
        Any,
        CorsLayer,
    },
    trace::TraceLayer,
};

use super::{
    extractors::{
        APIVersion,
        Expanded,
        MessagesQuery,
        OutputsQuery,
        Pagination,
        TimeRange,
    },
    responses::{
        ListenerResponse,
        Record,
        Transaction,
        Transfer,
    },
    ListenerError,
};
use crate::types::{
    message::{
        Message,
        MessageRecord,
    },
    mongo::BsonExt,
    sync::{
        SyncData,
        SyncRecord,
    },
    LedgerInclusionState,
};

type ListenerResult = Result<ListenerResponse, ListenerError>;

pub fn routes(database: Database) -> Router {
    #[allow(unused_mut)]
    let mut api_routes = Router::new().route("/info", get(info)).route("/sync", get(sync)).nest(
        "/:ver",
        Router::new()
            .nest(
                "/messages",
                Router::new()
                    .route("/", get(messages_query))
                    .route("/:message_id", get(message))
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
            .nest("/analytics", Router::new().route("/addresses", get(address_analytics))),
    );
    #[cfg(feature = "api-metrics")]
    {
        api_routes = api_routes.nest("/", super::metrics::routes());
        api_routes = api_routes.layer(super::metrics::MetricsLayer);
    }
    Router::new()
        .nest("/api", api_routes)
        .fallback(not_found.into_service())
        .layer(Extension(database))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(vec![Method::GET, Method::OPTIONS])
                .allow_headers(Any)
                .allow_credentials(true),
        )
}

async fn info() -> ListenerResult {
    let version = std::env!("CARGO_PKG_VERSION").to_string();
    let is_healthy = true;
    Ok(ListenerResponse::Info {
        name: "Chronicle".into(),
        version,
        is_healthy,
    })
}

async fn sync(database: Extension<Database>) -> Result<Json<SyncData>, ListenerError> {
    let mut res = database
        .collection::<SyncRecord>("sync")
        .find(
            doc! { "synced": true },
            FindOptions::builder().sort(doc! {"milestone_index": 1}).build(),
        )
        .await?;
    let mut sync_data = SyncData::default();
    let mut last_record: Option<SyncRecord> = None;
    while let Some(sync_record) = res.try_next().await? {
        // Missing records go into gaps
        if let Some(last) = last_record.as_ref() {
            if last.milestone_index + 1 != sync_record.milestone_index {
                sync_data
                    .gaps
                    .push(last.milestone_index + 1..sync_record.milestone_index - 1);
            }
        }
        // Synced AND logged records go into completed
        if sync_record.logged {
            match sync_data.completed.last_mut() {
                Some(last) => {
                    if last.end + 1 == sync_record.milestone_index {
                        last.end += 1;
                    } else {
                        sync_data
                            .completed
                            .push(sync_record.milestone_index..sync_record.milestone_index);
                    }
                }
                None => sync_data
                    .completed
                    .push(sync_record.milestone_index..sync_record.milestone_index),
            }
        // Otherwise the are synced only
        } else {
            match sync_data.synced_but_unlogged.last_mut() {
                Some(last) => {
                    if last.end + 1 == sync_record.milestone_index {
                        last.end += 1;
                    } else {
                        sync_data
                            .synced_but_unlogged
                            .push(sync_record.milestone_index..sync_record.milestone_index);
                    }
                }
                None => sync_data
                    .synced_but_unlogged
                    .push(sync_record.milestone_index..sync_record.milestone_index),
            }
        }
        last_record.replace(sync_record);
    }
    Ok(Json(sync_data))
}

async fn message(
    database: Extension<Database>,
    Path((_ver, message_id)): Path<(APIVersion, String)>,
) -> ListenerResult {
    let rec = MessageRecord::try_from(
        database
            .collection::<Document>("messages")
            .find_one(doc! {"message_id": &message_id.to_string()}, None)
            .await?.ok_or(ListenerError::NoResults)?,
    )?;
    Ok(ListenerResponse::Message {
        network_id: match &rec.message {
            Message::Chrysalis(m) => Some(m.network_id()),
            Message::Stardust(_) => None,
        },
        protocol_version: match &rec.message {
            Message::Chrysalis(_) => 0,
            Message::Stardust(m) => m.protocol_version(),
        },
        parents: rec.parents().map(|m| m.to_string()).collect(),
        payload: match &rec.message {
            Message::Chrysalis(m) => m.payload().as_ref().map(serde_json::to_value),
            Message::Stardust(m) => m.payload().map(serde_json::to_value),
        }
        .transpose()
        .map_err(|e| ListenerError::Other(e.into()))?,
        nonce: rec.nonce(),
    })
}

async fn message_metadata(
    database: Extension<Database>,
    Path((_ver, message_id)): Path<(APIVersion, String)>,
) -> ListenerResult {
    let rec = MessageRecord::try_from(
        database
            .collection::<Document>("messages")
            .find_one(doc! {"message_id": &message_id.to_string()}, None)
            .await?.ok_or(ListenerError::NoResults)?,
    )?;

    Ok(ListenerResponse::MessageMetadata {
        message_id: rec.message_id().to_string(),
        parent_message_ids: rec.message.parents().map(|id| id.to_string()).collect(),
        is_solid: rec.inclusion_state.is_some(),
        referenced_by_milestone_index: rec.inclusion_state.and(rec.milestone_index),
        milestone_index: rec.inclusion_state.and(rec.milestone_index),
        should_promote: Some(rec.inclusion_state.is_none()),
        should_reattach: Some(rec.inclusion_state.is_none()),
        ledger_inclusion_state: rec.inclusion_state.map(Into::into),
        conflict_reason: rec.conflict_reason().map(|c| *c as u8),
    })
}

async fn message_children(
    database: Extension<Database>,
    Path((_ver, message_id)): Path<(APIVersion, String)>,
    Pagination { page_size, page }: Pagination,
    Expanded { expanded }: Expanded,
) -> ListenerResult {
    let messages = database
        .collection::<Document>("messages")
        .find(
            doc! {"message.parents": &message_id.to_string()},
            FindOptions::builder()
                .skip((page_size * page) as u64)
                .sort(doc! {"milestone_index": -1})
                .limit(page_size as i64)
                .build(),
        )
        .await?
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .map(MessageRecord::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListenerResponse::MessageChildren {
        message_id: message_id.to_string(),
        max_results: page_size,
        count: messages.len(),
        children_message_ids: messages
            .into_iter()
            .map(|record| {
                if expanded {
                    Record {
                        id: record.message_id().to_string(),
                        inclusion_state: record.inclusion_state,
                        milestone_index: record.milestone_index,
                    }
                    .into()
                } else {
                    record.message_id().to_string().into()
                }
            })
            .collect(),
    })
}

async fn start_milestone(database: &Database, start_timestamp: OffsetDateTime) -> anyhow::Result<i32> {
    database
        .collection::<Document>("messages")
        .find(
            doc! {"message.payload.essence.timestamp": { "$gte": DateTime::from_millis(start_timestamp.unix_timestamp() * 1000) }},
            FindOptions::builder()
                .sort(doc! {"milestone_index": 1})
                .limit(1)
                .build(),
        )
        .await?
        .try_next()
        .await?
        .map(|mut d| {
            d.get_document_mut("message")
                .unwrap()
                .get_document_mut("payload")
                .unwrap()
                .get_document_mut("essence")
                .unwrap()
                .remove("index")
                .unwrap()
                .as_i32()
                .unwrap()
        })
        .ok_or_else(|| anyhow::anyhow!("No milestones found in time range"))
}

async fn end_milestone(database: &Database, end_timestamp: OffsetDateTime) -> anyhow::Result<i32> {
    database
        .collection::<Document>("messages")
        .find(
            doc! {"message.payload.essence.timestamp": { "$lte": DateTime::from_millis(end_timestamp.unix_timestamp() * 1000) }},
            FindOptions::builder()
                .sort(doc! {"milestone_index": -1})
                .limit(1)
                .build(),
        )
        .await?
        .try_next()
        .await?
        .map(|mut d| {
            d.get_document_mut("message")
                .unwrap()
                .get_document_mut("payload")
                .unwrap()
                .get_document_mut("essence")
                .unwrap()
                .remove("index")
                .unwrap()
                .as_i32()
                .unwrap()
        })
        .ok_or_else(|| anyhow::anyhow!("No milestones found in time range"))
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
) -> ListenerResult {
    let MessagesQuery { index, tag } = &query;
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let mut query_doc = doc! { "milestone_index": { "$gt": start_milestone, "$lt": end_milestone } };
    if let Some(index) = index.as_ref() {
        query_doc.insert("message.payload.index", index);
    }
    if let Some(tag) = tag.as_ref() {
        query_doc.insert("message.payload.tag", tag);
    }

    let messages = database
        .collection::<Document>("messages")
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
        .await?
        .into_iter()
        .map(MessageRecord::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListenerResponse::MessagesForQuery {
        query,
        max_results: page_size,
        count: messages.len(),
        message_ids: messages
            .into_iter()
            .map(|record| {
                if expanded {
                    Record {
                        id: record.message_id().to_string(),
                        inclusion_state: record.inclusion_state,
                        milestone_index: record.milestone_index,
                    }
                    .into()
                } else {
                    record.message_id().to_string().into()
                }
            })
            .collect(),
    })
}

async fn output(
    database: Extension<Database>,
    Path((_ver, transaction_id, idx)): Path<(APIVersion, String, u16)>,
) -> ListenerResult {
    let mut output = database
        .collection::<Document>("messages")
        .aggregate(
            vec![
                doc! { "$match": { "message.payload.transaction_id": &transaction_id.to_string() } },
                doc! { "$unwind": { "path": "$message.payload.essence.outputs", "includeArrayIndex": "message.payload.essence.outputs.idx" } },
                doc! { "$match": { "message.payload.essence.outputs.idx": idx as i64 } },
            ],
            None,
        )
        .await?
        .try_next()
        .await?.ok_or(ListenerError::NoResults)?;

    let spending_transaction = database
        .collection::<Document>("messages")
        .find_one(
            doc! {
                "inclusion_state": LedgerInclusionState::Included as u8 as i32,
                "message.payload.essence.inputs.transaction_id": &transaction_id.to_string(),
                "message.payload.essence.inputs.index": idx as i64
            },
            None,
        )
        .await?;

    Ok(ListenerResponse::Output {
        message_id: output.get_str("message_id").unwrap().to_owned(),
        transaction_id: transaction_id.to_string(),
        output_index: idx,
        spending_transaction: spending_transaction.map(|mut d| d.remove("message").unwrap().into()),
        output: output
            .get_document_mut("message")
            .unwrap()
            .get_document_mut("payload")
            .unwrap()
            .get_document_mut("essence")
            .unwrap()
            .remove("outputs")
            .unwrap()
            .into(),
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
) -> ListenerResult {
    let OutputsQuery {
        address,

        requires_dust_return: _,
        sender: _,
        tag: _,
        included,
    } = &query;
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let mut query_doc =
        vec![doc! { "$match": { "milestone_index": { "$gt": start_milestone, "$lt": end_milestone } } }];

    if let Some(address) = address.as_ref() {
        query_doc.extend([
            doc! { "$match": { "message.payload.essence.outputs.address.data": address } },
            doc! { "$set": {
                "message.payload.essence.outputs": {
                    "$filter": {
                        "input": "$message.payload.essence.outputs",
                        "as": "output",
                        "cond": { "$eq": [ "$$output.address.data", address ] }
                    }
                }
            } },
        ]);
    }
    query_doc.extend([
        doc! { "$unwind": { "path": "$message.payload.essence.outputs", "includeArrayIndex": "message.payload.essence.outputs.idx" } },
        doc! { "$sort": { "milestone_index": -1 } },
        doc! { "$skip": (page_size * page) as i64 },
        doc! { "$limit": page_size as i64 },
    ]);
    if *included {
        query_doc[0]
            .get_document_mut("$match")
            .unwrap()
            .insert("inclusion_state", LedgerInclusionState::Included as u8 as i32);
    }

    let outputs = database
        .collection::<Document>("messages")
        .aggregate(query_doc, None)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(ListenerResponse::OutputsForQuery {
        query,
        max_results: page_size,
        count: outputs.len(),
        output_ids: outputs
            .into_iter()
            .map(|record| {
                let payload = record.get_document("message").unwrap().get_document("payload").unwrap();
                let transaction_id =
                    crate::cpt2::prelude::TransactionId::from_str(payload.get_str("transaction_id").unwrap()).unwrap();
                let idx = payload
                    .get_document("essence")
                    .unwrap()
                    .get_document("outputs")
                    .unwrap()
                    .get("idx")
                    .unwrap()
                    .as_u16()
                    .unwrap();
                let output_id = crate::cpt2::prelude::OutputId::new(transaction_id, idx).unwrap();
                if expanded {
                    let inclusion_state = record
                        .get_i32("inclusion_state")
                        .ok()
                        .map(|s| LedgerInclusionState::try_from(s as u8).unwrap());
                    let milestone_index = record.get_i32("milestone_index").ok().map(|m| m as u32);
                    Record {
                        id: output_id.to_string(),
                        inclusion_state,
                        milestone_index,
                    }
                    .into()
                } else {
                    output_id.to_string().into()
                }
            })
            .collect(),
    })
}

async fn transaction_history(
    database: Extension<Database>,
    Path((_ver, address)): Path<(APIVersion, String)>,
    Pagination { page_size, page }: Pagination,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ListenerResult {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let records = database
        .collection::<Document>("messages")
        .aggregate(vec![
            // Only outputs for this address
            doc! { "$match": {
                "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                "inclusion_state": LedgerInclusionState::Included as u8 as i32, 
                "message.payload.essence.outputs.address.data": &address 
            } },
            doc! { "$set": {
                "message.payload.essence.outputs": {
                    "$filter": {
                        "input": "$message.payload.essence.outputs",
                        "as": "output",
                        "cond": { "$eq": [ "$$output.address.data", &address ] }
                    }
                }
            } },
            // One result per output
            doc! { "$unwind": { "path": "$message.payload.essence.outputs", "includeArrayIndex": "message.payload.essence.outputs.idx" } },
            // Lookup spending inputs for each output, if they exist
            doc! { "$lookup": {
                "from": "messages",
                // Keep track of the output id
                "let": { "transaction_id": "$message.payload.transaction_id", "index": "$message.payload.essence.outputs.idx" },
                "pipeline": [
                    // Match using the output's index
                    { "$match": { 
                        "inclusion_state": LedgerInclusionState::Included as u8 as i32, 
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
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let transactions = records
        .into_iter()
        .map(|rec| {
            let payload = rec.get_document("message").unwrap().get_document("payload").unwrap();
            let spending_transaction = rec.get_document("spending_transaction").ok();
            let output = payload
                .get_document("essence")
                .unwrap()
                .get_document("outputs")
                .unwrap();
            Transfer {
                transaction_id: payload.get_str("transaction_id").unwrap().to_owned(),
                output_index: output.get("idx").unwrap().as_u16().unwrap(),
                is_spending: spending_transaction.is_some(),
                inclusion_state: rec
                    .get_i32("inclusion_state")
                    .ok()
                    .map(|s| LedgerInclusionState::try_from(s as u8).unwrap()),
                message_id: rec.get_str("message_id").unwrap().to_owned(),
                amount: output.get_i64("amount").unwrap() as u64,
            }
        })
        .collect();

    Ok(ListenerResponse::TransactionHistory { transactions, address })
}

async fn transaction_for_message(
    database: Extension<Database>,
    Path((_ver, message_id)): Path<(APIVersion, String)>,
) -> ListenerResult {
    let transaction = MessageRecord::try_from(
        database
            .collection::<Document>("messages")
            .find_one(doc! {"message_id": &message_id}, None)
            .await?.ok_or(ListenerError::NoResults)?,
    )?;

    Ok(ListenerResponse::Transaction(Transaction {
        message_id,
        milestone_index: transaction.milestone_index,
        outputs: match &transaction.message {
            Message::Chrysalis(m) => match m.payload() {
                Some(crate::cpt2::payload::Payload::Transaction(t)) => match t.essence() {
                    crate::cpt2::prelude::Essence::Regular(e) => {
                        e.outputs().iter().map(|o| serde_json::to_value(o).unwrap()).collect()
                    }
                },
                _ => unreachable!(),
            },
            Message::Stardust(m) => match m.payload() {
                Some(crate::stardust::payload::Payload::Transaction(t)) => match t.essence() {
                    crate::stardust::payload::transaction::TransactionEssence::Regular(e) => {
                        e.outputs().iter().map(|o| serde_json::to_value(o).unwrap()).collect()
                    }
                },
                _ => unreachable!(),
            },
        },
        inputs: match &transaction.message {
            Message::Chrysalis(m) => match m.payload() {
                Some(crate::cpt2::payload::Payload::Transaction(t)) => match t.essence() {
                    crate::cpt2::prelude::Essence::Regular(e) => {
                        e.inputs().iter().map(|o| serde_json::to_value(o).unwrap()).collect()
                    }
                },
                _ => unreachable!(),
            },
            Message::Stardust(m) => match m.payload() {
                Some(crate::stardust::payload::Payload::Transaction(t)) => match t.essence() {
                    crate::stardust::payload::transaction::TransactionEssence::Regular(e) => {
                        e.inputs().iter().map(|o| serde_json::to_value(o).unwrap()).collect()
                    }
                },
                _ => unreachable!(),
            },
        },
    }))
}

async fn transaction_included_message(
    database: Extension<Database>,
    Path((_ver, transaction_id)): Path<(APIVersion, String)>,
) -> ListenerResult {
    let rec = MessageRecord::try_from(
        database
            .collection::<Document>("messages")
            .find_one(
                doc! {
                    "inclusion_state": LedgerInclusionState::Included as u8 as i32,
                    "message.payload.transaction_id": &transaction_id.to_string(),
                },
                None,
            )
            .await?.ok_or(ListenerError::NoResults)?,
    )?;

    Ok(ListenerResponse::Message {
        network_id: match &rec.message {
            Message::Chrysalis(m) => Some(m.network_id()),
            Message::Stardust(_) => None,
        },
        protocol_version: match &rec.message {
            Message::Chrysalis(_) => 0,
            Message::Stardust(m) => m.protocol_version(),
        },
        parents: rec.parents().map(|m| m.to_string()).collect(),
        payload: match &rec.message {
            Message::Chrysalis(m) => m.payload().as_ref().map(serde_json::to_value),
            Message::Stardust(m) => m.payload().map(serde_json::to_value),
        }
        .transpose()
        .map_err(|e| ListenerError::Other(e.into()))?,
        nonce: rec.nonce(),
    })
}

async fn milestone(database: Extension<Database>, Path((_ver, index)): Path<(APIVersion, u32)>) -> ListenerResult {
    database
        .collection::<Document>("messages")
        .find_one(doc! {"message.payload.essence.index": &index}, None)
        .await?.ok_or(ListenerError::NoResults)
        .and_then(|d| {
            let rec = MessageRecord::try_from(d)?;
            Ok(ListenerResponse::Milestone {
                milestone_index: index,
                message_id: rec.message_id.to_string(),
                timestamp: match &rec.message {
                    Message::Chrysalis(m) => {
                        if let Some(crate::cpt2::payload::Payload::Milestone(m)) = m.payload() {
                            m.essence().timestamp()
                        } else {
                            unreachable!()
                        }
                    }
                    Message::Stardust(m) => {
                        if let Some(crate::stardust::payload::Payload::Milestone(m)) = m.payload() {
                            m.essence().timestamp()
                        } else {
                            unreachable!()
                        }
                    }
                },
            })
        })
}

async fn address_analytics(
    database: Extension<Database>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ListenerResult {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let res = database
        .collection::<Document>("messages")
        .aggregate(
            vec![
                doc! { "$match": {
                    "inclusion_state": LedgerInclusionState::Included as u8 as i32,
                    "milestone_index": { "$gt": start_milestone, "$lt": end_milestone },
                    "message.payload.kind": crate::cpt2::payload::transaction::TransactionPayload::KIND as i32,
                } },
                doc! { "$unwind": { "path": "$message.payload.essence.inputs", "includeArrayIndex": "message.payload.essence.inputs.idx" } },
                doc! { "$lookup": {
                    "from": "messages",
                    "let": { "transaction_id": "$message.payload.essence.inputs.transaction_id", "index": "$message.payload.essence.inputs.index" },
                    "pipeline": [
                        { "$match": { 
                            "inclusion_state": LedgerInclusionState::Included as u8 as i32, 
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
                doc! { "$set": { "send_address": "$spent_transaction.message.payload.essence.outputs.address.data" } },
                doc! { "$unwind": { "path": "$message.payload.essence.outputs", "includeArrayIndex": "message.payload.essence.outputs.idx" } },
                doc! { "$set": { "recv_address": "$message.payload.essence.outputs.address.data" } },
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
        .await?.try_next().await?.ok_or_else(|| anyhow::anyhow!("No transactions found in time range"))?;

    Ok(ListenerResponse::AddressAnalytics {
        total_addresses: res.get("total_addresses").unwrap().as_u64().unwrap(),
        recv_addresses: res.get("recv_addresses").unwrap().as_u64().unwrap(),
        send_addresses: res.get("send_addresses").unwrap().as_u64().unwrap(),
    })
}

#[allow(unused)]
#[allow(unused_variables)]
async fn transactions_analytics(
    database: Extension<Database>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ListenerResult {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    todo!()
}

async fn not_found() -> ListenerError {
    ListenerError::NotFound
}
