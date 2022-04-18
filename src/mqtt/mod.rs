// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod error;

pub use error::MqttError;
use rumqttc::{AsyncClient, EventLoop, MqttOptions};
use serde::{Deserialize, Serialize};

/// A builder to establish a connection to INX.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MqttConfig {
    host: String,
    port: u16,
    cap: usize,
}

impl MqttConfig {
    /// Creates a new [`InxConfig`]. The `address` is the address of the node's INX interface.
    pub fn new(host: impl Into<String>, port: u16, cap: usize) -> Self {
        Self {
            host: host.into(),
            port,
            cap,
        }
    }

    /// Constructs an [`InxClient`] by consuming the [`InxConfig`].
    pub fn build(&self, client_id: impl Into<String>) -> Result<(AsyncClient, EventLoop), MqttError> {
        Ok(AsyncClient::new(
            MqttOptions::new(client_id, &self.host, self.port),
            self.cap,
        ))
    }
}
