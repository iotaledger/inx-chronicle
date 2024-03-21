// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// The different errors that can happen with database access.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum DbError {
    #[error("bson serialization error: {0}")]
    BsonSerialization(#[from] mongodb::bson::ser::Error),
    #[error("bson deserialization error: {0}")]
    BsonDeserialization(#[from] mongodb::bson::de::Error),
    #[error("mongodb error: {0}")]
    MongoDb(#[from] mongodb::error::Error),
    #[error("SDK block error: {0}")]
    SdkBlock(#[from] iota_sdk::types::block::BlockError),
    #[error("SDK mana error: {0}")]
    SdkMana(#[from] iota_sdk::types::block::mana::ManaError),
    #[error("missing record: {0}")]
    MissingRecord(String),
}
