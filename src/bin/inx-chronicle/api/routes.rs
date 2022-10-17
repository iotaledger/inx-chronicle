// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use auth_helper::jwt::{BuildValidation, Claims, JsonWebToken, Validation};
use axum::{
    handler::Handler,
    headers::{authorization::Bearer, Authorization},
    middleware::from_extractor,
    routing::{get, post},
    Extension, Json, TypedHeader,
};
use chronicle::{
    db::{collections::MilestoneCollection, MongoDb},
    types::stardust::milestone::MilestoneTimestamp,
};
use hyper::StatusCode;
use regex::RegexSet;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use super::{
    auth::Auth,
    config::ApiData,
    error::ApiError,
    extractors::ListRoutesQuery,
    responses::RoutesResponse,
    router::{RouteNode, Router},
    ApiResult,
};

const ALWAYS_AVAILABLE_ROUTES: &[&str] = &["/health", "/login", "/routes"];

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
        .route("/routes", get(list_routes))
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
    if password_verify(
        password.as_bytes(),
        config.password_salt.as_bytes(),
        &config.password_hash,
        Into::into(&config.argon_config),
    )
    .map_err(ApiError::internal)?
    {
        let jwt = JsonWebToken::new(
            Claims::new(ApiData::ISSUER, uuid::Uuid::new_v4().to_string(), ApiData::AUDIENCE)
                .map_err(ApiError::internal)?
                .expires_after_duration(config.jwt_expiration)
                .map_err(ApiError::internal)?,
            config.secret_key.as_ref(),
        )
        .map_err(ApiError::internal)?;

        Ok(format!("Bearer {}", jwt))
    } else {
        Err(ApiError::IncorrectPassword)
    }
}

/// Verifies if a password/salt pair matches a password hash.
pub fn password_verify(
    password: &[u8],
    salt: &[u8],
    hash: &[u8],
    config: argon2::Config,
) -> Result<bool, argon2::Error> {
    Ok(hash == argon2::hash_raw(password, salt, &config)?)
}

fn is_new_enough(timestamp: MilestoneTimestamp) -> bool {
    // Panic: The milestone_timestamp is guaranteeed to be valid.
    let timestamp = OffsetDateTime::from_unix_timestamp(timestamp.0 as i64).unwrap();
    OffsetDateTime::now_utc() <= timestamp + STALE_MILESTONE_DURATION
}

async fn list_routes(
    ListRoutesQuery { depth }: ListRoutesQuery,
    Extension(config): Extension<ApiData>,
    Extension(root): Extension<RouteNode>,
    bearer_header: Option<TypedHeader<Authorization<Bearer>>>,
) -> ApiResult<RoutesResponse> {
    let depth = depth.or(Some(3));
    let routes = if let Some(TypedHeader(Authorization(bearer))) = bearer_header {
        let jwt = JsonWebToken(bearer.token().to_string());

        jwt.validate(
            Validation::default()
                .with_issuer(ApiData::ISSUER)
                .with_audience(ApiData::AUDIENCE)
                .validate_nbf(true),
            config.secret_key.as_ref(),
        )
        .map_err(ApiError::InvalidJwt)?;

        root.list_routes(None, depth)
    } else {
        let public_routes = RegexSet::new(
            ALWAYS_AVAILABLE_ROUTES
                .iter()
                .copied()
                .chain(config.public_routes.patterns().iter().map(String::as_str)),
        )
        .unwrap(); // Panic: Safe as we know previous regex compiled and ALWAYS_AVAILABLE_ROUTES is const
        root.list_routes(public_routes, depth)
    };
    Ok(RoutesResponse { routes })
}

pub async fn is_healthy(database: &MongoDb) -> ApiResult<bool> {
    #[cfg(feature = "stardust")]
    {
        let newest = match database
            .collection::<MilestoneCollection>()
            .get_newest_milestone()
            .await
            .map_err(ApiError::internal)?
        {
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
        tracing::error!("An error occured during health check: {e}");
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
