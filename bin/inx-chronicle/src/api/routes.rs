// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use auth_helper::jwt::{Claims, JsonWebToken};
use axum::{
    handler::Handler,
    middleware::from_extractor,
    routing::{get, post},
    Extension, Json, Router,
};
use chronicle::{db::MongoDb, types::stardust::milestone::MilestoneTimestamp};
use hyper::StatusCode;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use super::{auth::Auth, config::ApiData, error::ApiError};

// Similar to Hornet, we enforce that the latest known milestone is newer than 5 minutes. This should give Chronicle
// sufficient time to catch up with the node that it is connected too. The current milestone interval is 5 seconds.
const STALE_MILESTONE_DURATION: Duration = Duration::minutes(5);

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new();

    #[cfg(feature = "stardust")]
    {
        router = router.merge(super::stardust::routes())
    }

    Router::new()
        .route("/health", get(health))
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

fn is_new_enough(timestamp: MilestoneTimestamp) -> bool {
    // Panic: The milestone_timestamp is guaranteeed to be valid.
    let timestamp = OffsetDateTime::from_unix_timestamp(timestamp.0 as i64).unwrap();
    OffsetDateTime::now_utc() <= timestamp + STALE_MILESTONE_DURATION
}

pub async fn is_healthy(database: &MongoDb) -> Result<bool, ApiError> {
    #[cfg(feature = "stardust")]
    {
        let newest = match database.get_newest_milestone().await? {
            Some(last) => last,
            None => return Ok(false),
        };

        if !is_new_enough(newest.milestone_timestamp) {
            return Ok(false);
        }
    }

    Ok(true)
}

pub async fn health(database: Extension<MongoDb>) -> StatusCode {
    let handle_error = |e| {
        log::error!("An error occured during health check: {e}");
        false
    };

    if is_healthy(&database).await.unwrap_or_else(handle_error) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

pub async fn not_found() -> ApiError {
    ApiError::NotFound
}

pub async fn not_implemented() -> ApiError {
    ApiError::NotImplemented
}
