// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use super::listener::InxListenerError;

#[derive(Debug, Error)]
pub enum InxWorkerError {
    #[error("failed to establish connection: {0}")]
    ConnectionError(inx::tonic::Error),
    #[error("failed to answer")]
    FailedToAnswerRequest,
    #[error("expected INX address with format `http://<address>:<port>`, but found `{0}`")]
    InvalidAddress(String),
    #[error(transparent)]
    ListenerError(#[from] InxListenerError),
    #[error("the collector is not running")]
    MissingCollector,
    #[error(transparent)]
    ParsingAddressFailed(#[from] url::ParseError),
    #[error(transparent)]
    Read(#[from] inx::tonic::Status),
    #[error(transparent)]
    Runtime(#[from] crate::RuntimeError),
    #[error(transparent)]
    TransportFailed(#[from] inx::tonic::Error),
}
