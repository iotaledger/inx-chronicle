// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{
        Extension,
        Path,
    },
    http::Method,
    routing::*,
    Router,
};
use mongodb::{
    bson::{
        doc,
        Document,
    },
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
    responses::ListenerResponse,
    ListenerError,
    MetricsLayer,
    REGISTRY,
};
use crate::types::{
    Message,
    MessageId,
    MessageRecord,
};

type ListenerResult = Result<ListenerResponse, ListenerError>;

pub fn routes(database: Database) -> Router {
    let routes = Router::new()
        .route("/info", get(info))
        .route("/metrics", get(metrics))
        .route("/messages/:message_id", get(get_message))
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

async fn get_message(database: Extension<Database>, Path(message_id): Path<String>) -> ListenerResult {
    MessageId::from_str(&message_id).map_err(|e| ListenerError::BadParse(e.into()))?;
    let rec = MessageRecord::try_from(
        database
            .collection::<Document>("messages")
            .find_one(doc! {"message_id": &message_id}, None)
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
