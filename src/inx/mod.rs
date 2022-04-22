// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod error;

use inx::{client::InxClient, tonic::Channel};
use serde::{Deserialize, Serialize};

pub use self::error::InxError;

/// A builder to establish a connection to INX.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct InxConfig {
    /// The bind address of node's INX interface.
    pub address: String,
}

impl Default for InxConfig {
    fn default() -> Self {
        Self {
            address: "http://localhost:9029".into(),
        }
    }
}

impl InxConfig {
    /// Creates a new [`InxConfig`]. The `address` is the address of the node's INX interface.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
        }
    }

    /// Constructs an [`InxClient`] by consuming the [`InxConfig`].
    pub async fn build(&self) -> Result<InxClient<Channel>, InxError> {
        let url = url::Url::parse(&self.address)?;

        if url.scheme() != "http" {
            return Err(InxError::InvalidAddress(self.address.clone()));
        }

        Ok(InxClient::connect(self.address.clone()).await?)
    }
}
