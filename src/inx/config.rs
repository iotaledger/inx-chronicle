// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// A builder to establish a connection to INX.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct InxConfig {
    pub(crate) connect_addr: String,
}

impl InxConfig {
    /// Creates a new [`InxConfig`]. The `address` is the address of the node's INX interface.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            connect_addr: address.into(),
        }
    }

    /// Returns the address of the endpoint the [`InxClient`] attempts to connect to.
    pub fn connect_addr(&self) -> &str {
        &self.connect_addr
    }
}
