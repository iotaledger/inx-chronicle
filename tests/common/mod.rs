// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use chronicle::db::{MongoDb, MongoDbCollection, MongoDbConfig};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TestDbError {
    #[error("failed to read config at '{0}': {1}")]
    FileRead(String, std::io::Error),
    #[error("toml deserialization failed: {0}")]
    TomlDeserialization(toml::de::Error),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
}

#[allow(unused)]
pub async fn connect_to_test_db(database_name: impl ToString) -> Result<MongoDb, TestDbError> {
    let mut config = if let Ok(path) = std::env::var("CONFIG_PATH") {
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
    config.database_name = database_name.to_string();

    Ok(MongoDb::connect(&config).await?)
}

#[allow(unused)]
pub async fn setup<T: MongoDbCollection + Send + Sync>(database_name: impl ToString) -> (MongoDb, T) {
    let db = connect_to_test_db(database_name).await.unwrap();
    db.clear().await.unwrap();
    db.create_indexes::<T>().await.unwrap();
    let collection = db.collection::<T>();
    (db, collection)
}

#[allow(unused)]
pub async fn teardown(db: MongoDb) {
    db.drop().await.unwrap();
}
