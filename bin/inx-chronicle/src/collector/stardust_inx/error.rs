// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use super::listener::StardustInxListenerError;

#[derive(Debug, Error)]
pub enum InxWorkerError {
    #[error("failed to establish connection: {0}")]
    ConnectionError(inx::tonic::Error),
    // TODO: use or remove dead code
    #[error("failed to answer")]
    #[allow(dead_code)]
    FailedToAnswerRequest,
    #[error("expected INX address with format `http://<address>:<port>`, but found `{0}`")]
    InvalidAddress(String),
    #[error(transparent)]
    ListenerError(#[from] StardustInxListenerError),
    #[error("the collector is not running")]
    MissingCollector,
    #[error(transparent)]
    ParsingAddressFailed(#[from] url::ParseError),
    #[error(transparent)]
    Read(#[from] inx::tonic::Status),
    #[error(transparent)]
    Runtime(#[from] chronicle::runtime::RuntimeError),
    #[error(transparent)]
    TransportFailed(#[from] inx::tonic::Error),
}
