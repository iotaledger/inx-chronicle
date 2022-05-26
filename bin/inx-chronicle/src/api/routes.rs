// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{
    handler::Handler,
    routing::{get, post},
    Extension, Router,
};
use chronicle::db::MongoDb;
use jsonwebtoken::{EncodingKey, Header};

use super::{config::ApiData, error::ApiError, responses::*, ApiResult};

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new()
        .route("/info", get(info))
        .route("/sync", get(sync))
        .route("/login", post(login));

    #[cfg(feature = "stardust")]
    {
        router = router.merge(super::stardust::routes())
    }

    Router::new().nest("/api", router).fallback(not_found.into_service())
}

async fn login(password_hash: String, Extension(config): Extension<ApiData>) -> Result<String, ApiError> {
    if password_hash == config.password_hash {
        let jwt = jsonwebtoken::encode(
            &Header::default(),
            &serde_json::json!({
                "iss": ApiData::ISSUER,
                "sub": uuid::Uuid::new_v4().to_string(),
                "aud": ApiData::AUDIENCE,
                "nbf": (time::OffsetDateTime::now_utc() - time::OffsetDateTime::UNIX_EPOCH).whole_seconds() as u64,
                "iat": (time::OffsetDateTime::now_utc() - time::OffsetDateTime::UNIX_EPOCH).whole_seconds() as u64
            }),
            &EncodingKey::from_secret(config.secret_key.as_ref()),
        )
        .map_err(|_| ApiError::Unauthorized)?;

        Ok(format!("BEARER {}", jwt))
    } else {
        Err(ApiError::Unauthorized)
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
