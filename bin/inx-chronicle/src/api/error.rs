// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{error::Error, str::ParseBoolError};

use axum::{extract::rejection::QueryRejection, response::IntoResponse};
use chronicle::{bson::DocError, db::model::inclusion_state::UnexpectedLedgerInclusionState};
use hyper::StatusCode;
use mongodb::bson::document::ValueAccessError;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum APIError {
    #[error("No results returned!")]
    NoResults,
    #[error("Provided index is too large! (Max 64 bytes)")]
    IndexTooLarge,
    #[error("Provided tag is too large! (Max 64 bytes)")]
    TagTooLarge,
    #[error("Invalid hexidecimal encoding!")]
    InvalidHex,
    #[error("No endpoint found!")]
    NotFound,
    #[error(transparent)]
    BadParse(ParseError),
    #[error(transparent)]
    QueryError(QueryRejection),
    #[error("Invalid time range!")]
    BadTimeRange,
    #[error(transparent)]
    ServerError(#[from] hyper::Error),
    #[error(transparent)]
    Other(#[from] Box<dyn Error + Send + Sync>),
}

impl APIError {
    /// Gets the HTTP status code associated with this error.
    pub fn status(&self) -> StatusCode {
        match self {
            APIError::NoResults | APIError::NotFound => StatusCode::NOT_FOUND,
            APIError::IndexTooLarge
            | APIError::TagTooLarge
            | APIError::InvalidHex
            | APIError::BadTimeRange
            | APIError::BadParse(_)
            | APIError::QueryError(_) => StatusCode::BAD_REQUEST,
            APIError::ServerError(_) | APIError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Gets the u16 status code representation associated with this error.
    pub fn code(&self) -> u16 {
        self.status().as_u16()
    }

    /// Creates a new APIError from a bad parse.
    pub fn bad_parse(err: impl Into<ParseError>) -> Self {
        APIError::BadParse(err.into())
    }

    /// Creates a new APIError from any error not accounted for.
    pub fn other(err: impl Error + Send + Sync + 'static) -> Self {
        APIError::Other(Box::new(err))
    }
}

impl IntoResponse for APIError {
    fn into_response(self) -> axum::response::Response {
        ErrorBody::from(self).into_response()
    }
}

macro_rules! impl_from_error {
    ($($t:ty),*) => {
        $(
            impl From<$t> for APIError {
                fn from(err: $t) -> Self {
                    Self::other(err)
                }
            }
        )*
    };
}
impl_from_error!(mongodb::error::Error);
impl_from_error!(ValueAccessError);
impl_from_error!(DocError);
impl_from_error!(UnexpectedLedgerInclusionState);
#[cfg(feature = "chrysalis")]
impl_from_error!(chronicle::chrysalis::Error);
#[cfg(feature = "stardust")]
impl_from_error!(chronicle::stardust::Error);

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(transparent)]
    TimeRange(#[from] time::error::ComponentRange),
    #[error(transparent)]
    Bool(#[from] ParseBoolError),
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

impl From<APIError> for ErrorBody {
    fn from(err: APIError) -> Self {
        Self {
            status: err.status(),
            code: err.code(),
            message: err.to_string(),
        }
    }
}
