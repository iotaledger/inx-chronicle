// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::tangle::MilestoneIndex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InxWorkerError {
    #[error("failed to establish connection")]
    ConnectionError,
    #[error("InfluxDb error {0}")]
    InfluxDb(#[from] influxdb::Error),
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
    SyncMilestoneGap { start: MilestoneIndex, end: MilestoneIndex },
    #[error("node confirmed milestone index `{node}` is less than index in database `{db}`")]
    SyncMilestoneIndexMismatch { node: MilestoneIndex, db: MilestoneIndex },
    #[error("INX error: {0}")]
    Inx(#[from] chronicle::inx::InxError),
}
