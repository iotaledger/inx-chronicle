// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// The different errors that can happen with INX.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum InxError {
    #[error("expected {expected} bytes but received {actual}")]
    InvalidByteLength { actual: usize, expected: usize },
    #[error("{0}")]
    InvalidRawBytes(String),
    #[error("missing field: {0}")]
    MissingField(&'static str),
    #[error("invalid enum variant: {0}")]
    InvalidVariant(&'static str),
    #[error("gRPC status code: {0}")]
    StatusCode(#[from] tonic::Status),
    #[error(transparent)]
    TonicError(#[from] tonic::transport::Error),
    #[error("SDK type error: {0}")]
    SDK(#[from] iota_sdk::types::block::Error),
}
