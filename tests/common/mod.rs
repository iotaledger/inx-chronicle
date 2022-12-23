// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::{MongoDb, MongoDbCollection, MongoDbConfig};

#[allow(unused)]
pub async fn setup_database(database_name: impl ToString) -> eyre::Result<MongoDb> {
    dotenvy::dotenv().ok();

    let mut test_config = MongoDbConfig {
        database_name: database_name.to_string(),
        ..Default::default()
    };

    if let Ok(conn_str) = std::env::var("MONGODB_CONN_STR") {
        test_config.conn_str = conn_str;
    };
    if let Ok(username) = std::env::var("MONGODB_USERNAME") {
        test_config.username = username;
    };
    if let Ok(password) = std::env::var("MONGODB_PASSWORD") {
        test_config.password = password;
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
