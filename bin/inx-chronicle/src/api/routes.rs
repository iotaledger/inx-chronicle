// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{handler::Handler, routing::get, Extension, Router};
use chronicle::db::{
    model::sync::{SyncData, SyncRecord},
    MongoDatabase,
};
use futures::TryStreamExt;
use hyper::Method;
use mongodb::{bson::doc, options::FindOptions, Database};
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use super::{error::ApiError, responses::*, ApiResult};

pub fn routes(db: MongoDatabase) -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new().route("/info", get(info)).route("/sync", get(sync));

    #[cfg(feature = "stardust")]
    {
        router = router.nest("/", super::stardust::routes())
    }

    Router::new()
        .nest("/api", router)
        .fallback(not_found.into_service())
        .layer(Extension(db))
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

async fn info() -> InfoResponse {
    let version = std::env!("CARGO_PKG_VERSION").to_string();
    let is_healthy = true;
    InfoResponse {
        name: "Chronicle".into(),
        version,
        is_healthy,
    }
}

async fn sync(database: Extension<Database>) -> ApiResult<SyncDataResponse> {
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
        // Otherwise they are synced only
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
    Ok(SyncDataResponse(sync_data))
}

async fn not_found() -> ApiError {
    ApiError::NotFound
}
