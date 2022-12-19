// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::{MongoDb, MongoDbCollection, MongoDbConfig};

#[allow(unused)]
pub async fn setup_database(database_name: impl ToString) -> eyre::Result<MongoDb> {
    dotenvy::dotenv().ok();

    let test_config = MongoDbConfig {
        conn_str: std::env::var("MONGODB_CONN_STR").unwrap_or_else(|_| "mongodb://localhost:27017".to_owned()),
        database_name: database_name.to_string(),
        username: std::env::var("MONGODB_USERNAME").unwrap_or_else(|_| "root".to_owned()),
        password: std::env::var("MONGODB_PASSWORD").unwrap_or_else(|_| "root".to_owned()),
        min_pool_size: 2,
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
