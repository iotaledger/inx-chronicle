// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{num::ParseIntError, str::ParseBoolError};

use axum::{
    extract::rejection::{QueryRejection, TypedHeaderRejection},
    response::IntoResponse,
};
use chronicle::db::collections::ParseSortError;
use hyper::{header::InvalidHeaderValue, StatusCode};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum InternalApiError {
    #[error("corrupt state: {0}")]
    CorruptState(&'static str),
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum ApiError {
    #[error(transparent)]
    BadParse(#[from] ParseError),
    #[error("invalid time range")]
    BadTimeRange,
    #[error("invalid password provided")]
    IncorrectPassword,
    #[error("internal server error")]
    Internal(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("invalid JWT provided: {0}")]
    InvalidJwt(auth_helper::jwt::Error),
    #[error("invalid authorization header provided: {0}")]
    InvalidAuthHeader(#[from] TypedHeaderRejection),
    #[error("no results returned")]
    NoResults,
    #[error("no endpoint found")]
    NotFound,
    #[error("endpoint not implemented")]
    NotImplemented,
    #[error(transparent)]
    QueryError(#[from] QueryRejection),
}

impl ApiError {
    /// Gets the HTTP status code associated with this error.
    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::NoResults | ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::BadTimeRange
            | ApiError::BadParse(_)
            | ApiError::InvalidAuthHeader(_)
            | ApiError::QueryError(_) => StatusCode::BAD_REQUEST,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::IncorrectPassword | ApiError::InvalidJwt(_) => StatusCode::UNAUTHORIZED,
            ApiError::NotImplemented => StatusCode::NOT_IMPLEMENTED,
        }
    }

    /// Gets the u16 status code representation associated with this error.
    pub fn code(&self) -> u16 {
        self.status().as_u16()
    }

    /// Creates a new ApiError from a bad parse.
    pub fn bad_parse(err: impl Into<ParseError>) -> Self {
        Self::BadParse(err.into())
    }

    pub fn internal(err: impl 'static + std::error::Error + Send + Sync) -> Self {
        Self::Internal(Box::new(err) as _)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        ErrorBody::from(self).into_response()
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("invalid cursor")]
    BadPagingState,
    #[cfg(feature = "stardust")]
    #[error(transparent)]
    BeeBlockStardust(#[from] bee_block_stardust::Error),
    #[error("invalid bool value provided: {0}")]
    Bool(#[from] ParseBoolError),
    #[error("invalid U256 value provided: {0}")]
    DecimalU256(#[from] uint::FromDecStrErr),
    #[error("invalid integer value provided: {0}")]
    Int(#[from] ParseIntError),
    #[error("invalid sort order provided: {0}")]
    SortOrder(#[from] ParseSortError),
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

impl From<ApiError> for ErrorBody {
    fn from(err: ApiError) -> Self {
        if let ApiError::Internal(e) = &err {
            error!("Internal API error: {}", e);
        }

        Self {
            status: err.status(),
            code: err.code(),
            message: err.to_string(),
        }
    }
}
