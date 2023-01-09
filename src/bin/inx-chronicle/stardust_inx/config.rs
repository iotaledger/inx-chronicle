use std::time::Duration;

use chronicle::types::tangle::MilestoneIndex;
use serde::{Deserialize, Serialize};

pub const DEFAULT_ENABLED: bool = true;
pub const DEFAULT_URL: &str = "http://localhost:9029";
pub const DEFAULT_RETRY_INTERVAL: &str = "5s";
pub const DEFAULT_RETRY_COUNT: usize = 30;
pub const DEFAULT_SYNC_START: u32 = 0;

/// Configuration for an INX connection.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InxConfig {
    pub enabled: bool,
    /// The bind address of node's INX interface.
    pub url: String,
    /// The time that has to pass until a new connection attempt is made.
    #[serde(with = "humantime_serde")]
    pub conn_retry_interval: Duration,
    /// The number of retries when connecting fails.
    pub conn_retry_count: usize,
    /// The milestone at which synchronization should begin.
    pub sync_start_milestone: MilestoneIndex,
}

impl Default for InxConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_ENABLED,
            url: DEFAULT_URL.to_string(),
            conn_retry_interval: DEFAULT_RETRY_INTERVAL.parse::<humantime::Duration>().unwrap().into(),
            conn_retry_count: DEFAULT_RETRY_COUNT,
            sync_start_milestone: DEFAULT_SYNC_START.into(),
        }
    }
}
