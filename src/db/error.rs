// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum MongoDbError {
    #[error("invalid client options: {0}")]
    InvalidClientOptions(mongodb::error::Error),
    #[error("insert failed: {0}")]
    InsertError(mongodb::error::Error)
}
