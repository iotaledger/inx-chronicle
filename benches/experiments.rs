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
    let output_collection = db.collection::<OutputCollection>();

    for milestone in 1..100 {
        let output_collection = &output_collection;
        let cone_stream = block_collection
            .get_referenced_blocks_in_white_flag_order_stream(milestone.into())
            .await
            .unwrap()
            .then(|block| async move {
                let mut input_res = None;
                if let Some(payload) = block.payload.as_ref() {
                    match payload {
                        Payload::Transaction(txn) => {
                            let TransactionEssence::Regular { inputs, .. } = &txn.essence;
                            input_res = Some(
                                futures::stream::iter(inputs.iter().filter_map(|input| match input {
                                    Input::Utxo(output_id) => Some(*output_id),
                                    _ => None,
                                }))
                                .then(|output_id| async move {
                                    output_collection.get_output(&output_id).await.unwrap().unwrap()
                                })
                                .collect::<Vec<_>>()
                                .await,
                            );
                        }
                        _ => (),
                    }
                }
                (block, input_res)
            });
        // BENCH
    }
}
