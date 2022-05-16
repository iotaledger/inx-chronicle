// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::syncer::SyncerConfig;

/// A builder to establish a connection to INX.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct InxConfig {
    /// The bind address of node's INX interface.
    pub connect_url: String,
    /// The time that has to pass until a new connection attempt is made.
    #[serde(with = "humantime_serde")]
    pub connection_retry_interval: Duration,
    pub syncer: SyncerConfig,
}

impl Default for InxConfig {
    fn default() -> Self {
        Self {
            connect_url: "http://localhost:9029".into(),
            connection_retry_interval: Duration::from_secs(5),
            syncer: Default::default(),
        }
    }
}

impl InxConfig {
    /// Creates a new [`InxConfig`]. The `address` is the address of the node's INX interface.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            connect_url: address.into(),
            ..Default::default()
        }
    }
}
