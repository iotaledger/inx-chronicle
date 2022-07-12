// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use auth_helper::jwt::{Claims, JsonWebToken};
use axum::{
    handler::Handler,
    middleware::from_extractor,
    routing::{get, post},
    Extension, Json, Router,
};
use chronicle::{db::MongoDb};
use hyper::StatusCode;
use serde::Deserialize;

use super::{auth::Auth, config::ApiData, error::ApiError, responses::*, ApiResult};

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new().route("/info", get(info)).route("/health", get(health));

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

async fn is_healthy(database: Extension<MongoDb>) -> bool {
    let first = database.find_first_milestone(0.into()).await;
    let last = database.find_last_milestone(u32::MAX.into()).await;

    if let (Ok(Some(start)), Ok(Some(end))) = (first, last) {
        if let Ok(sync) = database
            .get_sync_data(start.milestone_index..=end.milestone_index)
            .await
        {
            sync.gaps.len() == 0
        } else {
            false
        }
    } else {
        false
    }
}

async fn info(database: Extension<MongoDb>) -> InfoResponse {
    InfoResponse {
        name: "Chronicle".into(),
        version: std::env!("CARGO_PKG_VERSION").to_string(),
        is_healthy: is_healthy(database).await,
    }
}

pub async fn health(database: Extension<MongoDb>) -> StatusCode {
    if is_healthy(database).await {
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
