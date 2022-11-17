// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use chronicle::types::tangle::MilestoneIndex;
use serde::{Deserialize, Serialize};

/// Configuration for an INX connection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
}

impl InxConfig {
    /// Applies the corresponding user config.
    #[allow(clippy::option_map_unit_fn)]
    pub fn apply_user_config(&mut self, user_config: InxUserConfig) {
        user_config.enabled.map(|v| self.enabled = v);
        user_config.connect_url.map(|v| self.connect_url = v);
        user_config
            .connection_retry_interval
            .map(|v| self.connection_retry_interval = v);
        user_config
            .connection_retry_count
            .map(|v| self.connection_retry_count = v);
        user_config.sync_start_milestone.map(|v| self.sync_start_milestone = v);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InxUserConfig {
    pub enabled: Option<bool>,
    pub connect_url: Option<String>,
    #[serde(with = "humantime_serde")]
    pub connection_retry_interval: Option<Duration>,
    pub connection_retry_count: Option<usize>,
    pub sync_start_milestone: Option<MilestoneIndex>,
}
