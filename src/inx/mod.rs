// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod error;

use inx::{client::InxClient, Channel};
use serde::{Deserialize, Serialize};

pub use self::error::InxError;

/// A builder to establish a connection to INX.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct InxConfig {
    address: String,
}

impl InxConfig {
    /// Creates a new [`InxConfig`]. The `address` is the address of the node's INX interface.
    pub fn new<S: Into<String>>(address: S) -> Self {
        Self {
            address: address.into(),
        }
    }

    /// Constructs an [`InxClient`] by consuming the [`InxConfig`].
    pub async fn build(&self) -> Result<InxClient<Channel>, InxError> {
        InxClient::connect(self.address.clone())
            .await
            .map_err(|_| InxError::TransportFailed)
    }
}
