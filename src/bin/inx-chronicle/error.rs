// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use crate::config::ConfigError;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[cfg(feature = "influxdb")]
    #[error(transparent)]
    InfluxDb(#[from] influxdb::Error),
    #[cfg(feature = "api")]
    #[error(transparent)]
    Api(#[from] super::api::ApiError),
    #[cfg(feature = "inx")]
    #[error(transparent)]
    Inx(#[from] super::stardust_inx::InxWorkerError),
    #[error(transparent)]
    Shutdown(#[from] tokio::sync::broadcast::error::SendError<()>),
}
