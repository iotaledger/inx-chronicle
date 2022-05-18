// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::{Deserialize, Serialize};

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
    pub syncer: super::syncer::SyncerConfig,
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
