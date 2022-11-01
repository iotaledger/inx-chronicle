// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use auth_helper::jwt::{BuildValidation, JsonWebToken, Validation};
use axum::{
    extract::{FromRequest, OriginalUri},
    headers::{authorization::Bearer, Authorization},
    Extension, TypedHeader,
};

use super::{config::ApiData, ApiError, AuthError};

pub struct Auth;

#[async_trait]
impl<B: Send> FromRequest<B> for Auth {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        // Unwrap: <OriginalUri as FromRequest>::Rejection = Infallable
        let OriginalUri(uri) = OriginalUri::from_request(req).await.unwrap();

        let Extension(config) = Extension::<ApiData>::from_request(req).await?;

        if config.public_routes.is_match(&uri.to_string()) {
            return Ok(Auth);
        }

        let TypedHeader(Authorization(bearer)) = TypedHeader::<Authorization<Bearer>>::from_request(req).await?;
        let jwt = JsonWebToken(bearer.token().to_string());

        jwt.validate(
            Validation::default()
                .with_issuer(ApiData::ISSUER)
                .with_audience(ApiData::AUDIENCE)
                .validate_nbf(true),
            config.secret_key.as_ref(),
        )
        .map_err(AuthError::InvalidJwt)?;

        Ok(Auth)
    }
}
