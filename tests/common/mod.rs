// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use chronicle::db::{MongoDb, MongoDbCollection, MongoDbConfig};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TestDbError {
    #[error("failed to read config file at '{0}': {1}")]
    FileRead(String, std::io::Error),
    #[error("toml deserialization failed: {0}")]
    TomlDeserialization(toml::de::Error),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
}

#[allow(unused)]
pub async fn setup_database(database_name: impl ToString) -> Result<MongoDb, TestDbError> {
    let mut config: MongoDbConfig = {
        let path = "config.defaults.toml";
        let val = std::fs::read_to_string(path)
            .map_err(|e| TestDbError::FileRead(AsRef::<Path>::as_ref(path).display().to_string(), e))
            .and_then(|contents| toml::from_str::<toml::Value>(&contents).map_err(TestDbError::TomlDeserialization))?;
        // Panic: cannot fail because this section has to exist.
        let mongodb = val.get("mongodb").cloned().unwrap();
        mongodb.try_into().map_err(TestDbError::TomlDeserialization)?
    };
    config.database_name = database_name.to_string();

    let db = MongoDb::connect(&config).await?;
    db.clear().await?;
    Ok(db)
}

#[allow(unused)]
pub async fn setup_collection<T: MongoDbCollection + Send + Sync>(db: &MongoDb) -> Result<T, TestDbError> {
    db.create_indexes::<T>().await?;
    Ok(db.collection::<T>())
}

#[allow(unused)]
pub async fn teardown(db: MongoDb) {
    db.drop().await.unwrap();
}
