// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
mod error;

use inx::{client::InxClient, tonic::Channel};

pub use self::{config::InxConfig, error::InxError};

/// Creates an [`InxClient`] by connecting to the endpoint specified in `inx_config`.
pub async fn connect(inx_config: &InxConfig) -> Result<InxClient<Channel>, InxError> {
    Ok(InxClient::connect(inx_config.connect_addr.clone()).await?)
}
