// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::{collections::BlockCollection, MongoDb, MongoDbConfig};

#[tokio::test]
async fn test() {
    dotenvy::dotenv().ok();

    let config = MongoDbConfig {
        conn_str:
            "mongodb://dev-chronicle:password@localhost:27017/?authSource=admin&replicaSet=dbrs&directConnection=true"
                .to_string(),
        database_name: "chronicle_beta_25".to_string(),
    };

    let db = MongoDb::connect(&config).await.unwrap();
    let block_collection = db.collection::<BlockCollection>();

    for milestone in 1..100 {
        let cone_stream = block_collection
            .get_referenced_blocks_in_white_flag_order_stream(milestone.into())
            .await
            .unwrap();
        // BENCH
    }
}
