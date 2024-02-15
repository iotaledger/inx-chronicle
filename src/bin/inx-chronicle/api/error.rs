// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{num::ParseIntError, str::ParseBoolError};

use axum::{extract::rejection::QueryRejection, response::IntoResponse};
use axum_extra::typed_header::TypedHeaderRejection;
use chronicle::db::mongodb::collections::ParseSortError;
use hyper::{header::InvalidHeaderValue, StatusCode};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

/// The result of a request to the api
pub type ApiResult<T> = Result<T, ApiError>;

pub trait ErrorStatus: std::error::Error {
    /// Gets the HTTP status code associated with this error.
    fn status(&self) -> StatusCode;
}

#[derive(Debug, Error)]
#[allow(missing_docs)]
#[error("{code}: {error}")]
/// This type wraps errors that are associated with an HTTP status code.
pub struct ApiError {
    #[source]
    pub error: Box<dyn std::error::Error + Send + Sync>,
    code: StatusCode,
}

impl<T: 'static + ErrorStatus + Send + Sync> From<T> for ApiError {
    fn from(error: T) -> Self {
        Self {
            code: error.status(),
            error: Box::new(error) as _,
        }
    }
}

macro_rules! impl_internal_error {
    ($($type:ty),*) => {
        $(
            impl From<$type> for ApiError {
                fn from(error: $type) -> Self {
                    Self {
                        code: StatusCode::INTERNAL_SERVER_ERROR,
                        error: Box::new(error) as _,
                    }
                }
            }
        )*
    };
}

impl_internal_error!(
    mongodb::error::Error,
    chronicle::db::mongodb::DbError,
    chronicle::model::raw::InvalidRawBytesError,
    axum::extract::rejection::ExtensionRejection,
    auth_helper::jwt::Error,
    argon2::Error,
    iota_sdk::types::block::Error
);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        // Hide internal errors from the client, but print them to the server.
        let message = if self.code == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("Internal API error: {}", self.error);
            "internal server error".to_string()
        } else {
            self.error.to_string()
        };
        ErrorBody {
            status: self.code,
            code: self.code.as_u16(),
            message,
        }
        .into_response()
    }
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum CorruptStateError {
    // #[cfg(feature = "poi")]
    // #[error(transparent)]
    // PoI(#[from] crate::api::poi::CorruptStateError),
    #[error("no node configuration in the database")]
    NodeConfig,
    #[error("no protocol parameters in the database")]
    ProtocolParams,
}

impl ErrorStatus for CorruptStateError {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum AuthError {
    #[error("invalid password provided")]
    IncorrectPassword,
    #[error("invalid JWT provided: {0}")]
    InvalidJwt(auth_helper::jwt::Error),
}

impl ErrorStatus for AuthError {
    fn status(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
#[error("endpoint not implemented")]
pub struct UnimplementedError;

impl ErrorStatus for UnimplementedError {
    fn status(&self) -> StatusCode {
        StatusCode::NOT_IMPLEMENTED
    }
}

impl IntoResponse for UnimplementedError {
    fn into_response(self) -> axum::response::Response {
        ApiError::from(self).into_response()
    }
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum MissingError {
    #[error("no results returned")]
    NoResults,
    #[error("no endpoint found")]
    NotFound,
}

impl ErrorStatus for MissingError {
    fn status(&self) -> StatusCode {
        StatusCode::NOT_FOUND
    }
}

impl IntoResponse for MissingError {
    fn into_response(self) -> axum::response::Response {
        ApiError::from(self).into_response()
    }
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("invalid cursor")]
    BadPagingState,
    #[error("invalid time range")]
    BadTimeRange,

    #[error("invalid IOTA Stardust data: {0}")]
    IotaStardust(#[from] iota_sdk::types::block::Error),
    #[error("invalid bool value provided: {0}")]
    Bool(#[from] ParseBoolError),
    #[error("invalid U256 value provided: {0}")]
    DecimalU256(#[from] uint::FromDecStrErr),
    #[error("invalid hex value provided: {0}")]
    Hex(#[from] prefix_hex::Error),
    #[error("invalid integer value provided: {0}")]
    Int(#[from] ParseIntError),
    #[error("invalid authorization header provided: {0}")]
    InvalidAuthHeader(#[from] TypedHeaderRejection),
    #[error("invalid query parameters provided: {0}")]
    InvalidQueryParams(#[from] QueryRejection),
    // #[cfg(feature = "poi")]
    // #[error(transparent)]
    // PoI(#[from] crate::api::poi::RequestError),
    #[error("invalid sort order provided: {0}")]
    SortOrder(#[from] ParseSortError),
}

impl ErrorStatus for RequestError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("invalid allow-origin header in config: {0}")]
    InvalidHeader(#[from] InvalidHeaderValue),
    #[error("invalid hex value in config: {0}")]
    InvalidHex(#[from] hex::FromHexError),
    #[error("invalid regex in config: {0}")]
    InvalidRegex(#[from] regex::Error),
    #[error("invalid JWT config: {0}")]
    Jwt(#[from] argon2::Error),
    #[error("invalid secret key: {0}")]
    SecretKey(#[from] super::secret_key::SecretKeyError),
}

#[derive(Clone, Debug, Serialize)]
pub struct ErrorBody {
    #[serde(skip_serializing)]
    status: StatusCode,
    code: u16,
    message: String,
}

impl IntoResponse for ErrorBody {
    fn into_response(self) -> axum::response::Response {
        match serde_json::to_string(&self) {
            // Unwrap: Cannot fail as the only failure point is the header (which is valid).
            Ok(json) => axum::response::Response::builder()
                .status(self.status)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::new(json))
                .unwrap(),
            Err(e) => {
                error!("Unable to serialize error body: {}", e);
                Result::<(), _>::Err(format!("Unable to serialize error body: {e}")).into_response()
            }
        }
    }
}
