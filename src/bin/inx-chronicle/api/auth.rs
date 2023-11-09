// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use async_trait::async_trait;
use auth_helper::jwt::{BuildValidation, JsonWebToken, Validation};
use axum::{
    extract::{FromRef, FromRequestParts, OriginalUri},
    headers::{authorization::Bearer, Authorization},
    http::request::Parts,
    TypedHeader,
};

use super::{config::ApiConfigData, error::RequestError, ApiError, AuthError};

pub struct Auth;

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Auth
where
    Arc<ApiConfigData>: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Unwrap: <OriginalUri as FromRequest>::Rejection = Infallable
        let OriginalUri(uri) = OriginalUri::from_request_parts(parts, state).await.unwrap();

        let config = Arc::<ApiConfigData>::from_ref(state);

        if config.public_routes.is_match(&uri.to_string()) {
            return Ok(Auth);
        }

        let TypedHeader(Authorization(bearer)) = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        let jwt = JsonWebToken(bearer.token().to_string());

        jwt.validate(
            Validation::default()
                .with_issuer(ApiConfigData::ISSUER)
                .with_audience(ApiConfigData::AUDIENCE)
                .validate_nbf(true),
            config.jwt_secret_key.as_ref(),
        )
        .map_err(AuthError::InvalidJwt)?;

        Ok(Auth)
    }
}
