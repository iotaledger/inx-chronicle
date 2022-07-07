// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use auth_helper::jwt::{Claims, JsonWebToken};
use axum::{
    handler::Handler,
    middleware::from_extractor,
    routing::{get, post},
    Extension, Json, Router,
};
use chronicle::{db::MongoDb, runtime::ScopeView};
use hyper::StatusCode;
use serde::Deserialize;

use super::{auth::Auth, config::ApiData, error::ApiError, responses::*, ApiResult};

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new()
        .route("/info", get(info))
        .route("/health", get(health))
        .route("/routes", get(list_routes));

    #[cfg(feature = "stardust")]
    {
        router = router.merge(super::stardust::routes())
    }

    Router::new()
        .route("/login", post(login))
        .nest("/api", router.route_layer(from_extractor::<Auth>()))
        .fallback(not_found.into_service())
}

#[derive(Deserialize)]
struct LoginInfo {
    password: String,
}

async fn login(
    Json(LoginInfo { password }): Json<LoginInfo>,
    Extension(config): Extension<ApiData>,
) -> Result<String, ApiError> {
    if auth_helper::password::password_verify(
        password.as_bytes(),
        config.password_salt.as_bytes(),
        &config.password_hash,
    )? {
        let jwt = JsonWebToken::new(
            Claims::new(ApiData::ISSUER, uuid::Uuid::new_v4().to_string(), ApiData::AUDIENCE)?
                .expires_after_duration(config.jwt_expiration)?,
            config.secret_key.as_ref(),
        )?;

        Ok(format!("Bearer {}", jwt))
    } else {
        Err(ApiError::IncorrectPassword)
    }
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

async fn list_routes(Extension(config): Extension<ApiData>) -> RoutesResponse {
    RoutesResponse {
        // TODO: We should look at information from `axum::Router` to do this in a safer way. Also, we need a way to add
        // protected routes too, ideally while respecting the JWT.
        routes: config
            .public_routes
            .patterns()
            .iter()
            .map(|pattern| pattern.strip_suffix("/*").unwrap_or(pattern).to_owned())
            .collect(),
    }
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
