// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use thiserror::Error;

use crate::db::{mongodb::MongoClient, MongoDbConfig};

#[derive(Debug, Error)]
pub enum TestDbError {
    #[error("failed to read config at '{0}': {1}")]
    FileRead(String, std::io::Error),
    #[error("toml deserialization failed: {0}")]
    TomlDeserialization(toml::de::Error),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
}

pub(crate) async fn connect_to_test_db() -> Result<MongoClient, TestDbError> {
    let config = if let Ok(path) = std::env::var("CONFIG_PATH") {
        let val = std::fs::read_to_string(&path)
            .map_err(|e| TestDbError::FileRead(AsRef::<Path>::as_ref(&path).display().to_string(), e))
            .and_then(|contents| toml::from_str::<toml::Value>(&contents).map_err(TestDbError::TomlDeserialization))?;
        if let Some(mongodb) = val.get("mongodb").cloned() {
            mongodb.try_into().map_err(TestDbError::TomlDeserialization)?
        } else {
            MongoDbConfig::default()
        }
    } else {
        MongoDbConfig::default()
    };

    Ok(MongoClient::connect(&config).await?)
}
