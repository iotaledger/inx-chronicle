// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{extract::rejection::QueryRejection, response::IntoResponse};
use hyper::StatusCode;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
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
    BadParse(anyhow::Error),
    #[error(transparent)]
    QueryError(QueryRejection),
    #[error("Invalid time range!")]
    BadTimeRange,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl APIError {
    pub fn status(&self) -> StatusCode {
        match self {
            APIError::NoResults | APIError::NotFound => StatusCode::NOT_FOUND,
            APIError::IndexTooLarge
            | APIError::TagTooLarge
            | APIError::InvalidHex
            | APIError::BadTimeRange
            | APIError::BadParse(_)
            | APIError::QueryError(_) => StatusCode::BAD_REQUEST,
            APIError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn code(&self) -> u16 {
        self.status().as_u16()
    }

    pub fn bad_parse(err: impl Into<anyhow::Error>) -> Self {
        APIError::BadParse(err.into())
    }

    pub fn other(err: impl Into<anyhow::Error>) -> Self {
        APIError::Other(err.into())
    }
}

impl IntoResponse for APIError {
    fn into_response(self) -> axum::response::Response {
        ErrorBody::from(self).into_response()
    }
}

impl From<mongodb::error::Error> for APIError {
    fn from(e: mongodb::error::Error) -> Self {
        Self::Other(e.into())
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
