// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::future::Aborted;
use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("database error: {0}")]
    DatabaseError(#[from] mongodb::error::Error),
    // #[error("INX error: {0}")]
    // InxError(#[from] inx::proto::inx_client::Error),
    #[error("graceful shutdown failed")]
    ShutdownFailed,
    #[error(transparent)]
    Status(#[from] inx::Status),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Bson conversion error: {0}")]
    BsonError(#[from] mongodb::bson::ser::Error),
    #[error("Aborted")]
    Aborted(#[from] Aborted),
    #[error("Error: {0}")]
    Other(String),
}

impl Error {
    /// Create an error not defined by another variant
    pub fn other<S: Into<String>>(s: S) -> Self {
        Error::Other(s.into())
    }
}
