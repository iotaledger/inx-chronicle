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
}

#[allow(unused)]
pub async fn setup_database(database_name: impl ToString) -> eyre::Result<MongoDb> {
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

    let db = MongoDb::connect(&config, "Chronicle Test").await?;
    db.clear().await?;
    Ok(db)
}

#[allow(unused)]
pub async fn setup_collection<T: MongoDbCollection + Send + Sync>(db: &MongoDb) -> eyre::Result<T> {
    db.create_indexes::<T>().await?;
    Ok(db.collection::<T>())
}

#[allow(unused)]
pub async fn teardown(db: MongoDb) {
    db.drop().await.unwrap();
}
