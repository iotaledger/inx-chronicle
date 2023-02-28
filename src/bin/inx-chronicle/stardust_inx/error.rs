// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::{inx::InxError, types::stardust::tangle::milestone::MilestoneIndex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InxWorkerError {
    #[cfg(feature = "analytics")]
    #[error("Analytics error: {0}")]
    Analytics(#[from] chronicle::db::collections::analytics::Error),
    #[error("failed to establish connection: {0}")]
    ConnectionError(#[from] InxError),
    #[cfg(any(feature = "analytics", feature = "metrics"))]
    #[error("InfluxDb error: {0}")]
    InfluxDb(#[from] influxdb::Error),
    #[error("expected INX address with format `http://<address>:<port>`, but found `{0}`")]
    InvalidAddress(String),
    #[error("wrong number of ledger updates: `{received}` but expected `{expected}`")]
    InvalidLedgerUpdateCount { received: usize, expected: usize },
    #[error("invalid unspent output stream: found ledger index {found}, expected {expected}")]
    InvalidUnspentOutputIndex {
        found: MilestoneIndex,
        expected: MilestoneIndex,
    },
    #[error("invalid milestone state")]
    InvalidMilestoneState,
    #[error("missing milestone id for milestone index `{0}`")]
    MissingMilestoneInfo(MilestoneIndex),
    #[cfg(feature = "analytics")]
    #[error("missing application state")]
    MissingAppState,
    #[error("MongoDb error: {0}")]
    MongoDb(#[from] mongodb::error::Error),
    #[error("network changed from previous run. old network name: `{0}`, new network name: `{1}`")]
    NetworkChanged(String, String),
    #[error("node pruned required milestones between `{start}` and `{end}`")]
    SyncMilestoneGap { start: MilestoneIndex, end: MilestoneIndex },
    #[error("node confirmed milestone index `{node}` is less than index in database `{db}`")]
    SyncMilestoneIndexMismatch { node: MilestoneIndex, db: MilestoneIndex },
}
