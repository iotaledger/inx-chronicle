// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{num::ParseIntError, str::ParseBoolError};

use axum::{
    extract::rejection::{ExtensionRejection, QueryRejection, TypedHeaderRejection},
    response::IntoResponse,
};
use chronicle::runtime::ErrorLevel;
use hyper::{header::InvalidHeaderValue, StatusCode};
use mongodb::bson::document::ValueAccessError;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum InternalApiError {
    #[cfg(feature = "stardust")]
    #[error(transparent)]
    BeeStardust(#[from] bee_block_stardust::Error),
    #[error(transparent)]
    BsonDeserialize(#[from] mongodb::bson::de::Error),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    ExtensionRejection(#[from] ExtensionRejection),
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error(transparent)]
    Jwt(#[from] auth_helper::jwt::Error),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    PasswordHash(#[from] auth_helper::password::Error),
    #[error(transparent)]
    UrlEncoding(#[from] serde_urlencoded::de::Error),
    #[error(transparent)]
    ValueAccess(#[from] ValueAccessError),
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum ApiError {
    #[error(transparent)]
    BadParse(#[from] ParseError),
    #[error("Invalid time range")]
    BadTimeRange,
    #[error("Invalid password provided")]
    IncorrectPassword,
    #[error("Internal server error")]
    Internal(InternalApiError),
    #[error(transparent)]
    InvalidJwt(auth_helper::jwt::Error),
    #[error(transparent)]
    InvalidAuthHeader(#[from] TypedHeaderRejection),
    #[error("No results returned")]
    NoResults,
    #[error("No endpoint found")]
    NotFound,
    #[error("Endpoint not implemented")]
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
        ApiError::BadParse(err.into())
    }
}

impl ErrorLevel for ApiError {}

impl<T: Into<InternalApiError>> From<T> for ApiError {
    fn from(err: T) -> Self {
        ApiError::Internal(err.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        ErrorBody::from(self).into_response()
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[allow(dead_code)]
    #[error("Invalid cursor")]
    BadPagingState,
    #[error("Invalid sort order descriptor")]
    BadSortDescriptor,
    #[cfg(feature = "stardust")]
    #[error(transparent)]
    BeeBlockStardust(#[from] bee_block_stardust::Error),
    #[error(transparent)]
    Bool(#[from] ParseBoolError),
    #[error(transparent)]
    Int(#[from] ParseIntError),
    #[error(transparent)]
    DecimalU256(#[from] uint::FromDecStrErr),
    #[error(transparent)]
    TimeRange(#[from] time::error::ComponentRange),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    InvalidHeader(#[from] InvalidHeaderValue),
    #[error(transparent)]
    InvalidHex(#[from] hex::FromHexError),
    #[error("Invalid regex in config: {0}")]
    InvalidRegex(#[from] regex::Error),
    #[error(transparent)]
    SecretKey(#[from] super::secret_key::SecretKeyError),
    #[error(transparent)]
    TimeConversion(#[from] time::error::ConversionRange),
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
                log::error!("Unable to serialize error body: {}", e);
                Result::<(), _>::Err(format!("Unable to serialize error body: {}", e)).into_response()
            }
        }
    }
}

impl From<ApiError> for ErrorBody {
    fn from(err: ApiError) -> Self {
        if let ApiError::Internal(e) = &err {
            log::error!("Internal API error: {}", e);
        }

        Self {
            status: err.status(),
            code: err.code(),
            message: err.to_string(),
        }
    }
}
