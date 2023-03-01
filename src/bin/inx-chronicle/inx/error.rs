// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::model::block::payload::milestone::MilestoneIndex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InxWorkerError {
    #[error("expected INX address with format `http://<address>:<port>`, but found `{0}`")]
    InvalidAddress(String),
    #[error("invalid unspent output stream: found ledger index {found}, expected {expected}")]
    InvalidUnspentOutputIndex {
        found: MilestoneIndex,
        expected: MilestoneIndex,
    },
    #[cfg(feature = "analytics")]
    #[error("missing application state")]
    MissingAppState,
    #[error("network changed from previous run. old network name: `{old}`, new network name: `{new}`")]
    NetworkChanged { old: String, new: String },
    #[error("node pruned required milestones between `{start}` and `{end}`")]
    SyncMilestoneGap { start: MilestoneIndex, end: MilestoneIndex },
    #[error("node confirmed milestone index `{node}` is less than index in database `{db}`")]
    SyncMilestoneIndexMismatch { node: MilestoneIndex, db: MilestoneIndex },
}
