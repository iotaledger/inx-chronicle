// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{handler::Handler, routing::get, Extension, Router};
use chronicle::db::MongoDb;

use super::{error::ApiError, responses::*, ApiResult};

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new().route("/info", get(info)).route("/sync", get(sync));

    #[cfg(feature = "stardust")]
    {
        router = router.merge(super::stardust::routes())
    }

    Router::new().nest("/api", router).fallback(not_found.into_service())
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

async fn sync(database: Extension<MongoDb>) -> ApiResult<SyncDataResponse> {
    Ok(SyncDataResponse(database.get_sync_data(0..=u32::MAX).await?))
}

async fn not_found() -> ApiError {
    ApiError::NotFound
}
