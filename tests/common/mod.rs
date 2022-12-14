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
    dotenvy::dotenv().ok();

    let get_mongodb_test_config = || -> MongoDbConfig {
        MongoDbConfig {
            conn_str: std::env::var("MONGODB_CONN_STR").unwrap_or_else(|_| "mongodb://localhost:27017".to_owned()),
            database_name: database_name.to_string(),
            username: std::env::var("MONGODB_USERNAME").unwrap_or_else(|_| "root".to_owned()),
            password: std::env::var("MONGODB_PASSWORD").unwrap_or_else(|_| "root".to_owned()),
            min_pool_size: 2,
        }
    };

    let mut test_config = if let Ok(path) = std::env::var("CHRONICLE_TEST_CONFIG") {
        let val = std::fs::read_to_string(&path)
            .map_err(|e| TestDbError::FileRead(AsRef::<Path>::as_ref(&path).display().to_string(), e))
            .and_then(|contents| toml::from_str::<toml::Value>(&contents).map_err(TestDbError::TomlDeserialization))?;

        if let Some(mongodb) = val.get("mongodb").cloned() {
            let mut config: MongoDbConfig = mongodb.try_into().map_err(TestDbError::TomlDeserialization)?;
            config.database_name = database_name.to_string();
            config
        } else {
            get_mongodb_test_config()
        }
    } else {
        get_mongodb_test_config()
    };

    let db = MongoDb::connect(&test_config).await?;
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
