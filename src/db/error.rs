// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum MongoDbError {
    #[error("failed to serialize to BSON: {0}")]
    BsonSerializationError(#[from] mongodb::bson::ser::Error),
    #[error("database error: {0}")]
    DatabaseError(#[from] mongodb::error::Error),
}
