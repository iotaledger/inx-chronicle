// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::InxError;

const CONNECT_URL_DEFAULT: &str = "http://localhost:9029";

/// The INX client.
#[derive(Debug, Clone, Copy)]
pub struct InxClient;

impl InxClient {
    /// Creates an [`InxClient`] by consuming the builder.
    pub async fn connect(config: &InxClientConfig) -> Result<inx::client::InxClient<inx::tonic::Channel>, InxError> {
        Ok(inx::client::InxClient::connect(config.connect_url.clone()).await?)
    }
}

/// A builder to establish a connection to INX.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct InxClientConfig {
    pub(crate) connect_url: String,
}

impl InxClientConfig {
    /// Creates a new [`InxClient`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the URL of the INX endpoint.
    pub fn connect_url(&self) -> &str {
        &self.connect_url
    }

    /// Sets the connect URL.
    pub fn with_connect_url(mut self, connect_url: impl Into<String>) -> Self {
        self.connect_url = connect_url.into();
        self
    }

}

impl Default for InxClientConfig {
    fn default() -> Self {
        Self {
            connect_url: CONNECT_URL_DEFAULT.to_string(),
        }
    }
}
