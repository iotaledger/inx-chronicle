// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use chronicle::{
        db::{collections::ConfigurationUpdateCollection, MongoDbCollectionExt},
        types::node::{BaseToken, MilestoneKeyRange, NodeConfiguration},
    };

    use super::common::{setup_collection, setup_database, teardown};

    #[tokio::test]
    async fn test_node_configuration() {
        let db = setup_database("test-node-configuration").await.unwrap();
        let node_configuration = setup_collection::<ConfigurationUpdateCollection>(&db).await.unwrap();

        // empty collection
        assert!(
            node_configuration
                .get_latest_node_configuration()
                .await
                .unwrap()
                .is_none()
        );

        let mut config = NodeConfiguration {
            milestone_public_key_count: 3,
            milestone_key_ranges: vec![MilestoneKeyRange {
                public_key: "0xabcde".to_string(),
                start: 0.into(),
                end: 3.into(),
            }]
            .into_boxed_slice(),
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

        // correct insertion
        node_configuration
            .upsert_node_configuration(1.into(), config.clone())
            .await
            .unwrap();
        assert_eq!(node_configuration.count().await.unwrap(), 1);

        // correct fetch
        let latest_config = node_configuration
            .get_latest_node_configuration()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&latest_config.config.base_token.ticker_symbol, "SMR");

        // rejected upsert (same config)
        node_configuration
            .upsert_node_configuration(1.into(), config.clone())
            .await
            .unwrap();
        assert_eq!(node_configuration.count().await.unwrap(), 1);
        assert_eq!(&latest_config.config.base_token.name, "Shimmer");
        assert_eq!(&latest_config.config.base_token.ticker_symbol, "SMR");
        assert_eq!(&latest_config.config.base_token.unit, "SMR");
        assert_eq!(&latest_config.config.base_token.subunit, "glow");
        assert_eq!(latest_config.config.base_token.decimals, 6);
        assert!(!latest_config.config.base_token.use_metric_prefix);

        // accepted upsert (altered config)
        config.base_token.use_metric_prefix = true;
        node_configuration
            .upsert_node_configuration(1.into(), config.clone())
            .await
            .unwrap();
        assert_eq!(node_configuration.count().await.unwrap(), 1);

        let latest_config = node_configuration
            .get_latest_node_configuration()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&latest_config.config.base_token.ticker_symbol, "SMR");
        assert!(latest_config.config.base_token.use_metric_prefix);

        config.base_token.ticker_symbol = "SHI".to_string();
        config.base_token.unit = "suSHI".to_string();
        config.base_token.subunit = "rice".to_string();

        // accepted latest update
        node_configuration
            .upsert_node_configuration(2.into(), config)
            .await
            .unwrap();
        assert_eq!(node_configuration.count().await.unwrap(), 2);

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
        assert!(latest_config.config.base_token.use_metric_prefix);

        // get older update (yields the one inserted at index 1)
        let old_config = node_configuration
            .get_node_configuration_for_ledger_index(1.into())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&old_config.config.base_token.name, "Shimmer");
        assert_eq!(&old_config.config.base_token.ticker_symbol, "SMR");
        assert_eq!(&old_config.config.base_token.unit, "SMR");
        assert_eq!(&old_config.config.base_token.subunit, "glow");
        assert_eq!(old_config.config.base_token.decimals, 6);
        assert!(old_config.config.base_token.use_metric_prefix);

        teardown(db).await;
    }
}
