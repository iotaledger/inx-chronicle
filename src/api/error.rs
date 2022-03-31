// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{
    extract::rejection::QueryRejection,
    response::IntoResponse,
};
use hyper::StatusCode;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ListenerError {
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

impl ListenerError {
    pub fn status(&self) -> StatusCode {
        match self {
            ListenerError::NoResults | ListenerError::NotFound => StatusCode::NOT_FOUND,
            ListenerError::IndexTooLarge
            | ListenerError::InvalidHex
            | ListenerError::BadParse(_)
            | ListenerError::QueryError(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn code(&self) -> u16 {
        self.status().as_u16()
    }
}

impl IntoResponse for ListenerError {
    fn into_response(self) -> axum::response::Response {
        log::error!("{:?}", self);
        let err = ErrorBody::from(self);
        match serde_json::to_string(&err) {
            Ok(json) => axum::response::Response::builder()
                .status(err.status)
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

impl From<mongodb::error::Error> for ListenerError {
    fn from(e: mongodb::error::Error) -> Self {
        Self::Other(e.into())
    }
}

#[derive(Clone, Debug, Serialize)]
struct ErrorBody {
    #[serde(skip_serializing)]
    status: StatusCode,
    code: u16,
    message: String,
}

impl From<ListenerError> for ErrorBody {
    fn from(err: ListenerError) -> Self {
        Self {
            status: err.status(),
            code: err.code(),
            message: err.to_string(),
        }
    }
}
