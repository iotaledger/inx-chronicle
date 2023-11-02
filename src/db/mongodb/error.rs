// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// The different errors that can happen with database access.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum DbError {
    #[error("mongodb error: {0}")]
    MongoDb(#[from] mongodb::error::Error),
    #[error("SDK type error: {0}")]
    SDK(#[from] iota_sdk::types::block::Error),
}
