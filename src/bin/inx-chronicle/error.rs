// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use crate::{api::ApiError, config::ConfigError, stardust_inx::InxError};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[cfg(feature = "api")]
    #[error(transparent)]
    Api(#[from] ApiError),
    #[cfg(feature = "inx")]
    #[error(transparent)]
    Inx(#[from] InxError),
}
