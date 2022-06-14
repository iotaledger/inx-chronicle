// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::tangle::MilestoneIndex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InxError {
    #[error("failed to establish connection")]
    ConnectionError,
    #[error("expected INX address with format `http://<address>:<port>`, but found `{0}`")]
    InvalidAddress(String),
    #[error("INX type conversion error: {0:?}")]
    InxTypeConversion(#[from] inx::Error),
    #[error("missing milestone id for milestone index `{0}`")]
    MissingMilestoneId(MilestoneIndex),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error("network changed from previous run. old network name: {0}, new network name: {1}")]
    NetworkChanged(String, String),
    #[error(transparent)]
    ParsingAddressFailed(#[from] url::ParseError),
    #[error(transparent)]
    Read(#[from] inx::tonic::Status),
    #[error(transparent)]
    Runtime(#[from] chronicle::runtime::RuntimeError),
    #[error(transparent)]
    Tonic(#[from] inx::tonic::Error),
}
