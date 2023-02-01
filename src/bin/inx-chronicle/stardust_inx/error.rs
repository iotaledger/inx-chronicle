// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::{inx::InxError, types::tangle::MilestoneIndex};
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
    #[error("MongoDb error: {0}")]
    MongoDb(#[from] mongodb::error::Error),
    #[error("network changed from previous run. old network name: `{0}`, new network name: `{1}`")]
    NetworkChanged(String, String),
    #[error("node pruned required milestones between `{start}` and `{end}`")]
    SyncMilestoneGap { start: MilestoneIndex, end: MilestoneIndex },
    #[error("node confirmed milestone index `{node}` is less than index in database `{db}`")]
    SyncMilestoneIndexMismatch { node: MilestoneIndex, db: MilestoneIndex },
}
