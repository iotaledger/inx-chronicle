// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{
    extract::{
        Extension,
        Query,
    },
    http::Method,
    routing::*,
    Json,
    Router,
};
use chrono::NaiveDateTime;
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
use prometheus::{
    Encoder,
    TextEncoder,
};
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
        Index,
        Pagination,
        Tag,
        TimeRange,
    },
    responses::{
        ListenerResponse,
        Record,
    },
    ListenerError,
    MetricsLayer,
    REGISTRY,
};
use crate::types::{
    message::{
        Message,
        MessageId,
        MessageRecord,
    },
    sync::{
        SyncData,
        SyncRecord,
    },
};

type ListenerResult = Result<ListenerResponse, ListenerError>;

pub fn routes(database: Database) -> Router {
    let routes = Router::new()
        .route("/info", get(info))
        .route("/metrics", get(metrics))
        .route("/sync", get(sync))
        .route("/:ver/messages/:message_id", get(get_message))
        .route("/:ver/messages/:message_id/metadata", get(get_message_metadata))
        .route("/:ver/messages/:message_id/children", get(get_message_children))
        .route("/:ver/messages", get(get_message_query))
        .layer(Extension(database))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(MetricsLayer)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(vec![Method::GET, Method::OPTIONS])
                .allow_headers(Any)
                .allow_credentials(true),
        );

    Router::new().nest("/api", routes)
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

async fn metrics() -> Result<String, ListenerError> {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder
        .encode(&REGISTRY.gather(), &mut buffer)
        .map_err(|e| ListenerError::Other(e.into()))?;

    let res_custom = String::from_utf8(std::mem::take(&mut buffer)).map_err(|e| ListenerError::Other(e.into()))?;

    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .map_err(|e| ListenerError::Other(e.into()))?;

    let res_default = String::from_utf8(buffer).map_err(|e| ListenerError::Other(e.into()))?;

    Ok(format!("{}{}", res_custom, res_default))
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

async fn get_message(database: Extension<Database>, message_id: MessageId) -> ListenerResult {
    let rec = MessageRecord::try_from(
        database
            .collection::<Document>("messages")
            .find_one(doc! {"message_id": &message_id.to_string()}, None)
            .await?
            .ok_or_else(|| ListenerError::NoResults)?,
    )?;
    Ok(ListenerResponse::Message {
        network_id: match &rec.message {
            Message::Chrysalis(m) => Some(m.network_id()),
            Message::Shimmer(_) => None,
        },
        protocol_version: match &rec.message {
            Message::Chrysalis(_) => 0,
            Message::Shimmer(m) => m.protocol_version(),
        },
        parents: rec.parents().map(|m| m.to_string()).collect(),
        payload: match &rec.message {
            Message::Chrysalis(m) => m.payload().as_ref().map(|p| serde_json::to_value(p)),
            Message::Shimmer(m) => m.payload().map(|p| serde_json::to_value(p)),
        }
        .transpose()
        .map_err(|e| ListenerError::Other(e.into()))?,
        nonce: rec.nonce(),
    })
}

async fn get_message_metadata(database: Extension<Database>, message_id: MessageId) -> ListenerResult {
    let rec = MessageRecord::try_from(
        database
            .collection::<Document>("messages")
            .find_one(doc! {"message_id": &message_id.to_string()}, None)
            .await?
            .ok_or_else(|| ListenerError::NoResults)?,
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

async fn get_message_children(
    database: Extension<Database>,
    message_id: MessageId,
    Pagination { page_size, page }: Pagination,
    expanded: Option<Query<bool>>,
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
        .map(|d| MessageRecord::try_from(d))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListenerResponse::MessageChildren {
        message_id: message_id.to_string(),
        max_results: page_size,
        count: messages.len(),
        children_message_ids: messages
            .into_iter()
            .map(|record| {
                if let Some(Query(true)) = expanded {
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

async fn start_milestone(database: &Database, start_timestamp: NaiveDateTime) -> anyhow::Result<i32> {
    database
        .collection::<Document>("messages")
        .find(
            doc! {"message.payload.essence.timestamp": { "$gte": DateTime::from_millis(start_timestamp.timestamp_millis()) }},
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

async fn end_milestone(database: &Database, end_timestamp: NaiveDateTime) -> anyhow::Result<i32> {
    database
        .collection::<Document>("messages")
        .find(
            doc! {"message.payload.essence.timestamp": { "$lte": DateTime::from_millis(end_timestamp.timestamp_millis()) }},
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

async fn get_message_query(
    database: Extension<Database>,
    index: Result<Index, ListenerError>,
    tag: Result<Tag, ListenerError>,
    Pagination { page_size, page }: Pagination,
    expanded: Option<Query<bool>>,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ListenerResult {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let mut query = doc! { "milestone_index": { "$gt": start_milestone, "$lt": end_milestone } };
    if let Err(ListenerError::InvalidHex) = index.as_ref() {
        return Err(ListenerError::InvalidHex);
    }
    if let Err(ListenerError::InvalidHex) = tag.as_ref() {
        return Err(ListenerError::InvalidHex);
    }
    if let Ok(Index { index }) = index.as_ref() {
        query.insert("message.payload.index", index);
    }
    if let Ok(Tag { tag }) = tag.as_ref() {
        query.insert("message.payload.tag", tag);
    }

    let messages = database
        .collection::<Document>("messages")
        .find(
            query,
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
        .map(|d| MessageRecord::try_from(d))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListenerResponse::MessagesForQuery {
        index: index.ok().map(|i| i.index),
        tag: tag.ok().map(|i| i.tag),
        max_results: page_size,
        count: messages.len(),
        message_ids: messages
            .into_iter()
            .map(|record| {
                if let Some(Query(true)) = expanded {
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
