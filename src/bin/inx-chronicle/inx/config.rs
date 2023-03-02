// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::model::MilestoneIndex;

pub const DEFAULT_ENABLED: bool = true;
pub const DEFAULT_URL: &str = "http://localhost:9029";
pub const DEFAULT_SYNC_START: u32 = 0;

/// Configuration for an INX connection.
#[derive(Clone, Debug)]
pub struct InxConfig {
    pub enabled: bool,
    /// The bind address of node's INX interface.
    pub url: String,
    /// The milestone at which synchronization should begin.
    pub sync_start_milestone: MilestoneIndex,
}

impl Default for InxConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_ENABLED,
            url: DEFAULT_URL.to_string(),
            sync_start_milestone: DEFAULT_SYNC_START.into(),
        }
    }
}
