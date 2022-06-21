// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{handler::Handler, routing::get, Extension, Router};
use chronicle::db::MongoDb;
use hyper::StatusCode;

use super::{error::ApiError, responses::*, ApiResult};

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new().route("/info", get(info)).route("/health", get(health));

    #[cfg(feature = "stardust")]
    {
        router = router.merge(super::stardust::routes())
    }

    Router::new().nest("/api", router).fallback(not_found.into_service())
}

fn is_healthy() -> bool {
    true
}

async fn info() -> InfoResponse {
    let version = std::env!("CARGO_PKG_VERSION").to_string();
    let is_healthy = is_healthy();
    InfoResponse {
        name: "Chronicle".into(),
        version,
        is_healthy,
    }
}

pub async fn health() -> StatusCode {
    match is_healthy() {
        true => StatusCode::OK,
        false => StatusCode::SERVICE_UNAVAILABLE,
    }
}

pub async fn sync(database: Extension<MongoDb>) -> ApiResult<SyncDataDto> {
    Ok(SyncDataDto(database.get_sync_data(0.into()..=u32::MAX.into()).await?))
}

pub async fn not_found() -> ApiError {
    ApiError::NotFound
}

pub async fn not_implemented() -> ApiError {
    ApiError::NotImplemented
}
