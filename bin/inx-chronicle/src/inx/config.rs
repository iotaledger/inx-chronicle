// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use inx::{client::InxClient, tonic::Channel};
use serde::{Deserialize, Serialize};

pub use super::InxWorkerError;

/// A builder to establish a connection to INX.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct InxConfig {
    /// The bind address of node's INX interface.
    pub address: String,
    /// The time that has to pass until a new connection attempt is made.
    pub connection_retry_interval: Duration,
}

impl Default for InxConfig {
    fn default() -> Self {
        Self {
            address: "http://localhost:9029".into(),
            connection_retry_interval: Duration::from_secs(5),
        }
    }
}

impl InxConfig {
    /// Creates a new [`InxConfig`]. The `address` is the address of the node's INX interface.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            ..Default::default()
        }
    }

    /// Constructs an [`InxClient`] by consuming the [`InxConfig`].
    pub async fn build(&self) -> Result<InxClient<Channel>, InxWorkerError> {
        let url = url::Url::parse(&self.address)?;

        if url.scheme() != "http" {
            return Err(InxWorkerError::InvalidAddress(self.address.clone()));
        }

        InxClient::connect(self.address.clone())
            .await
            .map_err(InxWorkerError::ConnectionError)
    }
}
