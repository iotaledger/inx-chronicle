// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::tangle::MilestoneIndex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InxWorkerError {
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
    #[error("network changed from previous run. old network name: `{0}`, new network name: `{1}`")]
    NetworkChanged(String, String),
    #[error("node pruned required milestones between `{start}` and `{end}`")]
    SyncMilestoneGap { start: MilestoneIndex, end: MilestoneIndex },
    #[error("node confirmed milestone index `{node}` is less than index in database `{db}`")]
    SyncMilestoneIndexMismatch { node: MilestoneIndex, db: MilestoneIndex },
}
