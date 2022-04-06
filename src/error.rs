// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database error: {0}")]
    DatabaseError(#[from] mongodb::error::Error),
    // #[error("INX error: {0}")]
    // InxError(#[from] inx::proto::inx_client::Error),
    #[error("graceful shutdown failed")]
    ShutdownFailed,
}
