// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use chronicle::types::tangle::MilestoneIndex;
use serde::{Deserialize, Serialize};

/// Configuration for an INX connection.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct InxConfig {
    pub enabled: bool,
    /// The bind address of node's INX interface.
    pub connect_url: String,
    /// The time that has to pass until a new connection attempt is made.
    #[serde(with = "humantime_serde")]
    pub connection_retry_interval: Duration,
    /// The number of retries when connecting fails.
    pub connection_retry_count: usize,
    /// The milestone at which synchronization should begin.
    pub sync_start_milestone: MilestoneIndex,
    /// The interval at which to report syncing progress.
    pub sync_report_interval: Duration,
}

impl Default for InxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            connect_url: "http://localhost:9029".into(),
            connection_retry_interval: Duration::from_secs(5),
            connection_retry_count: 5,
            sync_start_milestone: 1.into(),
            sync_report_interval: Duration::from_secs(5),
        }
    }
}
