// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{handler::Handler, routing::get, Extension, Router};
use chronicle::{db::MongoDb, runtime::ScopeView};
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

async fn is_healthy(#[allow(unused)] scope: &ScopeView) -> bool {
    #[allow(unused_mut)]
    let mut is_healthy = true;
    #[cfg(feature = "inx")]
    {
        use crate::check_health::CheckHealth;
        is_healthy &= scope
            .is_healthy::<crate::stardust_inx::InxWorker>()
            .await
            .unwrap_or(false);
    }
    is_healthy
}

async fn info(Extension(scope): Extension<ScopeView>) -> InfoResponse {
    InfoResponse {
        name: "Chronicle".into(),
        version: std::env!("CARGO_PKG_VERSION").to_string(),
        is_healthy: is_healthy(&scope).await,
    }
}

pub async fn health(Extension(scope): Extension<ScopeView>) -> StatusCode {
    if is_healthy(&scope).await {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

#[cfg(feature = "api-history")]
pub async fn sync(database: Extension<MongoDb>) -> ApiResult<SyncDataDto> {
    Ok(SyncDataDto(database.get_sync_data(0.into()..=u32::MAX.into()).await?))
}

pub async fn not_found() -> ApiError {
    ApiError::NotFound
}

pub async fn not_implemented() -> ApiError {
    ApiError::NotImplemented
}
