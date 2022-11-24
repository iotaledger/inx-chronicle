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
use chronicle::db::MongoDb;
use hyper::StatusCode;
use regex::RegexSet;
use serde::Deserialize;

use super::{
    auth::Auth,
    config::ApiData,
    error::{ApiError, MissingError, UnimplementedError},
    extractors::ListRoutesQuery,
    responses::RoutesResponse,
    router::{RouteNode, Router},
    ApiResult, AuthError,
};

const ALWAYS_AVAILABLE_ROUTES: &[&str] = &["/health", "/login", "/routes"];

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
) -> ApiResult<String> {
    if password_verify(
        password.as_bytes(),
        config.password_salt.as_bytes(),
        &config.password_hash,
        Into::into(&config.argon_config),
    )? {
        let jwt = JsonWebToken::new(
            Claims::new(ApiData::ISSUER, uuid::Uuid::new_v4().to_string(), ApiData::AUDIENCE)?
                .expires_after_duration(config.jwt_expiration)?,
            config.secret_key.as_ref(),
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

pub async fn health(database: Extension<MongoDb>) -> StatusCode {
    let handle_error = |error| {
        tracing::error!("An error occured during health check: {error}");
        false
    };

    if database.is_healthy().await.unwrap_or_else(handle_error) {
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
