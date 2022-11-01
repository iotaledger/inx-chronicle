// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{num::ParseIntError, str::ParseBoolError};

use axum::{extract::rejection::TypedHeaderRejection, response::IntoResponse};
use chronicle::db::collections::ParseSortError;
use hyper::{header::InvalidHeaderValue, StatusCode};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

/// The result of a request to the api
pub type ApiResult<T> = Result<T, ApiError>;

pub trait ErrorStatus: std::error::Error {
    /// Gets the HTTP status code associated with this error.
    fn status(&self) -> StatusCode;

    /// Gets the u16 status code representation associated with this error.
    fn code(&self) -> u16 {
        self.status().as_u16()
    }
}

#[derive(Debug)]
#[allow(missing_docs)]
/// This type wraps errors that are associated with an HTTP status code.
/// It intentionally does not impl std::error::Error so that we can use
/// the impl From below to convert errors to internal 500 codes by default.
pub struct ApiError {
    pub error: Box<dyn std::error::Error + Send + Sync>,
    code: StatusCode,
}

impl<T: 'static + std::error::Error + Send + Sync> From<T> for ApiError {
    fn from(error: T) -> Self {
        Self {
            error: Box::new(error) as _,
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ApiError {
    pub fn new<T: 'static + ErrorStatus + Send + Sync>(error: T) -> Self {
        Self {
            code: error.status(),
            error: Box::new(error) as _,
        }
    }

    pub fn bad_request<T: 'static + std::error::Error + Send + Sync>(error: T) -> Self {
        Self {
            error: Box::new(error) as _,
            code: StatusCode::BAD_REQUEST,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        ErrorBody {
            status: self.code,
            code: self.code.as_u16(),
            message: self.error.to_string(),
        }
        .into_response()
    }
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum CorruptStateError {
    #[error("no milestone in the database")]
    NoMilestone,
    #[error("no protocol parameters in the database")]
    NoProtocolParams,
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
        ApiError::new(self).into_response()
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
        ApiError::new(self).into_response()
    }
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("invalid cursor")]
    BadPagingState,
    #[error("invalid time range")]
    BadTimeRange,
    #[cfg(feature = "stardust")]
    #[error(transparent)]
    BeeBlockStardust(#[from] bee_block_stardust::Error),
    #[error("invalid bool value provided: {0}")]
    Bool(#[from] ParseBoolError),
    #[error("invalid U256 value provided: {0}")]
    DecimalU256(#[from] uint::FromDecStrErr),
    #[error("invalid integer value provided: {0}")]
    Int(#[from] ParseIntError),
    #[error("invalid authorization header provided: {0}")]
    InvalidAuthHeader(#[from] TypedHeaderRejection),
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
    #[error("invalid secret key: {0}")]
    SecretKey(#[from] super::secret_key::SecretKeyError),
}

impl ErrorStatus for ConfigError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
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
                .body(axum::body::boxed(axum::body::Full::from(json)))
                .unwrap(),
            Err(e) => {
                error!("Unable to serialize error body: {}", e);
                Result::<(), _>::Err(format!("Unable to serialize error body: {}", e)).into_response()
            }
        }
    }
}
