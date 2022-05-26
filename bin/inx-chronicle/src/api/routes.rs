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

use super::{auth::Auth, config::ApiData, error::ApiError, responses::*, ApiResult};

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
    #[serde(rename = "passwordHash")]
    password_hash: String,
}

async fn login(
    Json(LoginInfo { password_hash }): Json<LoginInfo>,
    Extension(config): Extension<ApiData>,
) -> Result<String, ApiError> {
    if password_hash == config.password_hash {
        let now = time::OffsetDateTime::now_utc();
        let exp = now + config.jwt_expiration;
        let jwt = jsonwebtoken::encode(
            &Header::default(),
            &serde_json::json!({
                "iss": ApiData::ISSUER,
                "sub": uuid::Uuid::new_v4().to_string(),
                "aud": ApiData::AUDIENCE,
                "nbf": (now - time::OffsetDateTime::UNIX_EPOCH).whole_seconds() as u64,
                "iat": (now - time::OffsetDateTime::UNIX_EPOCH).whole_seconds() as u64,
                "exp": (exp - time::OffsetDateTime::UNIX_EPOCH).whole_seconds() as u64,
            }),
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

async fn sync(database: Extension<MongoDb>) -> ApiResult<SyncDataResponse> {
    Ok(SyncDataResponse(
        database.get_sync_data(0.into()..=u32::MAX.into()).await?,
    ))
}

async fn not_found() -> ApiError {
    ApiError::NotFound
}
