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
    pub sync_kind: SyncKind,
    pub max_parallel_requests: usize,
}

impl Default for InxConfig {
    fn default() -> Self {
        Self {
            connect_url: "http://localhost:9029".into(),
            connection_retry_interval: Duration::from_secs(5),
            sync_kind: Default::default(),
            max_parallel_requests: 10,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncKind {
    #[serde(rename = "max_milestones")]
    Max(u32),
    #[serde(rename = "from_milestone")]
    From(u32),
}

impl Default for SyncKind {
    fn default() -> Self {
        Self::From(1)
    }
}
