// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum InxWorkerError {
    #[error("failed to establish connection: {0}")]
    ConnectionError(inx::tonic::Error),
    #[error("failed to answer")]
    FailedToAnswerRequest,
    #[error("expected INX address with format `http://<address>:<port>`, but found `{0}`")]
    InvalidAddress(String),
    #[error("INX type conversion error: {0:?}")]
    InxTypeConversion(inx::Error),
    #[error(transparent)]
    ListenerError(#[from] super::listener::InxListenerError),
    #[error("the collector is not running")]
    MissingCollector,
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    ParsingAddressFailed(#[from] url::ParseError),
    #[error(transparent)]
    Read(#[from] inx::tonic::Status),
    #[error(transparent)]
    Runtime(#[from] chronicle::runtime::RuntimeError),
    #[error(transparent)]
    TransportFailed(#[from] inx::tonic::Error),
}
