// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, OriginalUri},
    headers::{authorization::Bearer, Authorization},
    Extension, TypedHeader,
};
use jsonwebtoken::{DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use super::{config::ApiData, ApiError};

pub struct Auth;

#[async_trait]
impl<B: Send> FromRequest<B> for Auth {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        // Unwrap: Infallable
        let OriginalUri(uri) = OriginalUri::from_request(req).await.unwrap();

        let Extension(config) = Extension::<ApiData>::from_request(req).await?;

        if config.public_routes.is_match(&uri.to_string()) {
            return Ok(Auth);
        }

        let TypedHeader(Authorization(bearer)) = TypedHeader::<Authorization<Bearer>>::from_request(req).await?;
        let jwt = bearer.token().to_string();

        let mut validation = Validation::default();
        validation.set_issuer(&[ApiData::ISSUER]);
        validation.set_audience(&[ApiData::AUDIENCE]);
        validation.validate_nbf = true;

        jsonwebtoken::decode::<Claims>(&jwt, &DecodingKey::from_secret(config.secret_key.as_ref()), &validation)
            .map_err(ApiError::InvalidJwt)?;

        Ok(Auth)
    }
}

/// Represents registered JSON Web Token Claims.
/// <https://tools.ietf.org/html/rfc7519#section-4.1>
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Issuer.
    /// Identifies the principal that issued the JWT. The processing of this claim is generally application specific.
    iss: String,
    /// Subject.
    /// Identifies the principal that is the subject of the JWT. The claims in a JWT are normally statements about the
    /// subject. The subject value MUST either be scoped to be locally unique in the context of the issuer or be
    /// globally unique. The processing of this claim is generally application specific.
    sub: String,
    /// Audience.
    /// Identifies the recipients that the JWT is intended for. Each principal intended to process the JWT MUST
    /// identify itself with a value in the audience claim. If the principal processing the claim does not identify
    /// itself with a value in the "aud" claim when this claim is present, then the JWT MUST be rejected. The
    /// interpretation of audience values is generally application specific.
    aud: String,
    /// Expiration Time.
    /// Identifies the expiration time on or after which the JWT MUST NOT be accepted for processing. The processing of
    /// the "exp" claim requires that the current date/time MUST be before the expiration date/time listed in the "exp"
    /// claim. Implementers MAY provide for some small leeway, usually no more than a few minutes, to account for clock
    /// skew.
    #[serde(skip_serializing_if = "Option::is_none")]
    exp: Option<u64>,
    /// Not Before.
    /// Identifies the time before which the JWT MUST NOT be accepted for processing. The processing of the "nbf" claim
    /// requires that the current date/time MUST be after or equal to the not-before date/time listed in the "nbf"
    /// claim. Implementers MAY provide for some small leeway, usually no more than a few minutes, to account for clock
    /// skew.
    nbf: u64,
    /// Issued At.
    /// Identifies the time at which the JWT was issued. This claim can be used to determine the age of the JWT.
    iat: u64,
}
