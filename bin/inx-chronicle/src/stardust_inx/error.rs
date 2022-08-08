// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::{runtime::ErrorLevel, types::tangle::MilestoneIndex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InxError {
    #[error("failed to establish connection")]
    ConnectionError,
    #[error("expected INX address with format `http://<address>:<port>`, but found `{0}`")]
    InvalidAddress(String),
    #[error("wrong number of ledger updates: `{received}` but expected `{expected}`")]
    InvalidLedgerUpdateCount { received: usize, expected: usize },
    #[error("invalid milestone state")]
    InvalidMilestoneState,
    #[error("missing milestone id for milestone index `{0}`")]
    MissingMilestoneInfo(MilestoneIndex),
    #[error("MongoDB error: {0}")]
    MongoDb(#[from] mongodb::error::Error),
    #[error("network changed from previous run. old network name: `{0}`, new network name: `{1}`")]
    NetworkChanged(String, String),
    #[error(transparent)]
    ParsingAddressFailed(#[from] url::ParseError),
    #[error("node pruned required milestones between `{start}` and `{end}`")]
    MilestoneGap { start: MilestoneIndex, end: MilestoneIndex },
    #[error(transparent)]
    Runtime(#[from] chronicle::runtime::RuntimeError),
    #[error("INX error: {0}")]
    BeeInx(#[from] bee_inx::Error),
}

impl ErrorLevel for InxError {
    fn level(&self) -> log::Level {
        match self {
            Self::InvalidAddress(_) | Self::MongoDb(_) | Self::NetworkChanged(_, _) | Self::ParsingAddressFailed(_) => {
                log::Level::Error
            }
            _ => log::Level::Warn,
        }
    }
}
