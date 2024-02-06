// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use auth_helper::jwt::{BuildValidation, Claims, JsonWebToken, Validation};
use axum::{
    extract::State,
    http::HeaderValue,
    middleware::from_extractor_with_state,
    routing::{get, post},
    Json,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chronicle::db::{
    mongodb::collections::{ApplicationStateCollection, CommittedSlotCollection},
    MongoDb,
};
use hyper::StatusCode;
use regex::RegexSet;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use super::{
    auth::Auth,
    config::ApiConfigData,
    error::{ApiError, MissingError, UnimplementedError},
    extractors::ListRoutesQuery,
    responses::RoutesResponse,
    router::{RouteNode, Router},
    ApiResult, ApiState, AuthError,
};

pub(crate) static BYTE_CONTENT_HEADER: HeaderValue = HeaderValue::from_static("application/vnd.iota.serializer-v1");

const ALWAYS_AVAILABLE_ROUTES: &[&str] = &["/health", "/login", "/routes"];

// Similar to Hornet, we enforce that the latest known slot is newer than 5 minutes. This should give Chronicle
// sufficient time to catch up with the node that it is connected too.
const STALE_SLOT_DURATION: Duration = Duration::minutes(5);

pub fn routes(config: Arc<ApiConfigData>) -> Router<ApiState> {
    #[allow(unused_mut)]
    let mut router = Router::<ApiState>::new()
        .nest("/core/v3", super::core::routes())
        .nest("/explorer/v3", super::explorer::routes())
        .nest("/indexer/v2", super::indexer::routes());

    // #[cfg(feature = "poi")]
    // {
    //     router = router.nest("/poi/v1", super::poi::routes());
    // }

    Router::<ApiState>::new()
        .route("/health", get(health))
        .route("/login", post(login))
        .route("/routes", get(list_routes))
        .nest("/api", router.route_layer(from_extractor_with_state::<Auth, _>(config)))
        .fallback(get(not_found))
}

#[derive(Deserialize)]
struct LoginInfo {
    password: String,
}

async fn login(
    State(config): State<Arc<ApiConfigData>>,
    Json(LoginInfo { password }): Json<LoginInfo>,
) -> ApiResult<String> {
    if password_verify(
        password.as_bytes(),
        config.jwt_password_salt.as_bytes(),
        &config.jwt_password_hash,
        Into::into(&config.jwt_argon_config),
    )? {
        let jwt = JsonWebToken::new(
            Claims::new(
                ApiConfigData::ISSUER,
                uuid::Uuid::new_v4().to_string(),
                ApiConfigData::AUDIENCE,
            )?
            .expires_after_duration(config.jwt_expiration)?,
            config.jwt_secret_key.as_ref(),
        )?;

        Ok(format!("Bearer {jwt}"))
    } else {
        Err(ApiError::from(AuthError::IncorrectPassword))
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

fn is_new_enough(slot_timestamp: u64) -> bool {
    // Panic: The slot timestamp is guaranteeed to be valid.
    let timestamp = OffsetDateTime::from_unix_timestamp(slot_timestamp as _).unwrap();
    OffsetDateTime::now_utc() <= timestamp + STALE_SLOT_DURATION
}

async fn list_routes(
    ListRoutesQuery { depth }: ListRoutesQuery,
    State(config): State<Arc<ApiConfigData>>,
    State(root): State<Arc<RouteNode>>,
    bearer_header: Option<TypedHeader<Authorization<Bearer>>>,
) -> ApiResult<RoutesResponse> {
    let depth = depth.or(Some(3));
    let routes = if let Some(TypedHeader(Authorization(bearer))) = bearer_header {
        let jwt = JsonWebToken(bearer.token().to_string());

        jwt.validate(
            Validation::default()
                .with_issuer(ApiConfigData::ISSUER)
                .with_audience(ApiConfigData::AUDIENCE)
                .validate_nbf(true),
            config.jwt_secret_key.as_ref(),
        )
        .map_err(AuthError::InvalidJwt)?;

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
    {
        if let Some(newest_slot) = database
            .collection::<CommittedSlotCollection>()
            .get_latest_committed_slot()
            .await?
        {
            if let Some(protocol_params) = database
                .collection::<ApplicationStateCollection>()
                .get_protocol_parameters()
                .await?
            {
                if is_new_enough(newest_slot.slot_index.to_timestamp(
                    protocol_params.genesis_unix_timestamp(),
                    protocol_params.slot_duration_in_seconds(),
                )) {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

pub async fn health(database: State<MongoDb>) -> StatusCode {
    let handle_error = |ApiError { error, .. }| {
        tracing::error!("An error occured during health check: {error}");
        false
    };

    if is_healthy(&database).await.unwrap_or_else(handle_error) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

pub async fn not_found() -> MissingError {
    MissingError::NotFound
}

pub async fn not_implemented() -> UnimplementedError {
    UnimplementedError
}
