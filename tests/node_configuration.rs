// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use chronicle::{
        db::{collections::ConfigurationUpdateCollection, MongoDbCollectionExt},
        types::node::{BaseToken, NodeConfiguration},
    };

    use super::common::{setup_collection, setup_database, teardown};

    #[tokio::test]
    async fn test_node_configuration() {
        let db = setup_database("test-node-configuration").await.unwrap();
        let node_configuration = setup_collection::<ConfigurationUpdateCollection>(&db).await.unwrap();

        let mut config = NodeConfiguration {
            base_token: BaseToken {
                name: "Shimmer".to_string(),
                ticker_symbol: "SMR".to_string(),
                unit: "SMR".to_string(),
                subunit: "glow".to_string(),
                decimals: 6,
                use_metric_prefix: false,
            },
        };
        assert_eq!(node_configuration.count().await.unwrap(), 0);

        node_configuration
            .update_latest_node_configuration(0.into(), config.clone())
            .await
            .unwrap();
        assert_eq!(node_configuration.count().await.unwrap(), 1);

        let latest_config = node_configuration
            .get_latest_node_configuration()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&latest_config.config.base_token.ticker_symbol, "SMR");

        node_configuration
            .upsert_node_configuration(0.into(), latest_config.config.clone())
            .await
            .unwrap();
        assert_eq!(node_configuration.count().await.unwrap(), 1);
        assert_eq!(&latest_config.config.base_token.ticker_symbol, "SMR");

        config.base_token.ticker_symbol = "SHI".to_string();
        config.base_token.unit = "suSHI".to_string();
        config.base_token.subunit = "rice".to_string();

        node_configuration
            .upsert_node_configuration(0.into(), config)
            .await
            .unwrap();
        assert_eq!(node_configuration.count().await.unwrap(), 1);

        let latest_config = node_configuration
            .get_latest_node_configuration()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&latest_config.config.base_token.name, "Shimmer");
        assert_eq!(&latest_config.config.base_token.ticker_symbol, "SHI");
        assert_eq!(&latest_config.config.base_token.unit, "suSHI");
        assert_eq!(&latest_config.config.base_token.subunit, "rice");
        assert_eq!(latest_config.config.base_token.decimals, 6);
        assert!(!latest_config.config.base_token.use_metric_prefix);

        teardown(db).await;
    }
}
