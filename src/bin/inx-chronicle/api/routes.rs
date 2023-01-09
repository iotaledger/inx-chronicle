// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use auth_helper::jwt::{BuildValidation, Claims, JsonWebToken, Validation};
use axum::{
    extract::State,
    headers::{authorization::Bearer, Authorization},
    http::header::HeaderValue,
    middleware::from_extractor_with_state,
    routing::{get, post},
    Json, TypedHeader,
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
    config::ApiConfigData,
    error::{ApiError, MissingError, UnimplementedError},
    extractors::ListRoutesQuery,
    responses::RoutesResponse,
    router::{Router, RouterState},
    ApiResult, ApiWorker, AuthError,
};

pub(crate) static BYTE_CONTENT_HEADER: HeaderValue = HeaderValue::from_static("application/vnd.iota.serializer-v1");

const ALWAYS_AVAILABLE_ROUTES: &[&str] = &["/health", "/login", "/routes"];

// Similar to Hornet, we enforce that the latest known milestone is newer than 5 minutes. This should give Chronicle
// sufficient time to catch up with the node that it is connected too. The current milestone interval is 5 seconds.
const STALE_MILESTONE_DURATION: Duration = Duration::minutes(5);

pub fn routes(api_data: ApiConfigData) -> Router<ApiWorker> {
    #[allow(unused_mut)]
    let mut router = Router::new();

    {
        router = router.merge(super::stardust::routes())
    }

    Router::new()
        .route("/health", get(health))
        .route("/login", post(login))
        .route("/routes", get(list_routes))
        .nest(
            "/api",
            router.route_layer(from_extractor_with_state::<Auth, _>(api_data)),
        )
        .fallback(not_found)
}

#[derive(Deserialize)]
struct LoginInfo {
    password: String,
}

async fn login(
    State(config_data): State<ApiConfigData>,
    Json(LoginInfo { password }): Json<LoginInfo>,
) -> ApiResult<String> {
    if password_verify(
        password.as_bytes(),
        config_data.jwt_password_salt.as_bytes(),
        &config_data.jwt_password_hash,
        Into::into(&config_data.jwt_argon_config),
    )? {
        let jwt = JsonWebToken::new(
            Claims::new(
                ApiConfigData::ISSUER,
                uuid::Uuid::new_v4().to_string(),
                ApiConfigData::AUDIENCE,
            )?
            .expires_after_duration(config_data.jwt_expiration)?,
            config_data.jwt_secret_key.as_ref(),
        )?;

        Ok(format!("Bearer {}", jwt))
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

fn is_new_enough(timestamp: MilestoneTimestamp) -> bool {
    // Panic: The milestone_timestamp is guaranteeed to be valid.
    let timestamp = OffsetDateTime::from_unix_timestamp(timestamp.0 as i64).unwrap();
    OffsetDateTime::now_utc() <= timestamp + STALE_MILESTONE_DURATION
}

async fn list_routes(
    ListRoutesQuery { depth }: ListRoutesQuery,
    State(state): State<RouterState<ApiWorker>>,
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
            state.inner.api_data.jwt_secret_key.as_ref(),
        )
        .map_err(AuthError::InvalidJwt)?;

        state.routes.list_routes(None, depth)
    } else {
        let public_routes = RegexSet::new(
            ALWAYS_AVAILABLE_ROUTES
                .iter()
                .copied()
                .chain(state.inner.api_data.public_routes.patterns().iter().map(String::as_str)),
        )
        .unwrap(); // Panic: Safe as we know previous regex compiled and ALWAYS_AVAILABLE_ROUTES is const
        state.routes.list_routes(public_routes, depth)
    };
    Ok(RoutesResponse { routes })
}

pub async fn is_healthy(database: &MongoDb) -> ApiResult<bool> {
    {
        let newest = match database
            .collection::<MilestoneCollection>()
            .get_newest_milestone()
            .await?
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
