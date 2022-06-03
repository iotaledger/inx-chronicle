// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{
    handler::Handler,
    middleware::from_extractor,
    routing::{get, post},
    Extension, Json, Router,
};
use chronicle::db::MongoDb;
use jsonwebtoken::{EncodingKey, Header};
use serde::Deserialize;

use super::{
    auth::{Auth, Claims},
    config::ApiData,
    error::ApiError,
    responses::*,
    ApiResult,
};

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new().route("/info", get(info)).route("/sync", get(sync));

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
    #[serde(rename = "password")]
    password: String,
}

async fn login(
    Json(LoginInfo { password }): Json<LoginInfo>,
    Extension(config): Extension<ApiData>,
) -> Result<String, ApiError> {
    if argon2::verify_raw(
        password.as_bytes(),
        config.password_salt.as_bytes(),
        &config.password_hash,
        &argon2::Config::default(),
    )? {
        let now = time::OffsetDateTime::now_utc();
        let exp = now + config.jwt_expiration;
        let now_ts = (now - time::OffsetDateTime::UNIX_EPOCH).whole_seconds() as u64;
        let claims = Claims {
            iss: ApiData::ISSUER.into(),
            sub: uuid::Uuid::new_v4().to_string(),
            aud: ApiData::AUDIENCE.into(),
            exp: Some((exp - time::OffsetDateTime::UNIX_EPOCH).whole_seconds() as u64),
            nbf: now_ts,
            iat: now_ts,
        };
        let jwt = jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.secret_key.as_ref()),
        )?;

        Ok(format!("Bearer {}", jwt))
    } else {
        Err(ApiError::IncorrectPassword)
    }
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

async fn sync(database: Extension<MongoDb>) -> ApiResult<SyncDataDto> {
    Ok(SyncDataDto(database.get_sync_data(0.into()..=u32::MAX.into()).await?))
}

async fn not_found() -> ApiError {
    ApiError::NotFound
}
