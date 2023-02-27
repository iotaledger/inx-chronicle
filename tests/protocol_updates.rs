// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use chronicle::{
        db::{collections::ProtocolUpdateCollection, MongoDbCollectionExt},
        model::{stardust::payload::milestone::MilestoneIndex, tangle::ProtocolParameters},
    };
    use iota_types::block::rand::number::rand_number_range;

    use super::common::{setup_collection, setup_database, teardown};

    #[tokio::test]
    async fn test_protocol_updates() {
        let db = setup_database("test-protocol-updates").await.unwrap();
        let update_collection = setup_collection::<ProtocolUpdateCollection>(&db).await.unwrap();

        let mut update_indexes = vec![];

        for (ledger_index, parameters) in std::iter::repeat(())
            .enumerate()
            .step_by(rand_number_range(10..100usize))
            .take(10)
            .inspect(|(i, _)| update_indexes.push(MilestoneIndex(*i as u32)))
            .enumerate()
            .map(|(i, (ledger_index, _))| {
                let mut parameters = ProtocolParameters::from(iota_types::block::protocol::protocol_parameters());
                parameters.version = i as u8;
                (ledger_index as u32, parameters)
            })
        {
            update_collection
                .upsert_protocol_parameters(ledger_index.into(), parameters)
                .await
                .unwrap();
        }

        assert_eq!(update_collection.count().await.unwrap(), 10);
        assert_eq!(
            update_collection.get_latest_protocol_parameters().await.unwrap(),
            update_collection.get_protocol_parameters_for_version(9).await.unwrap()
        );
        for (version, index) in update_indexes.into_iter().enumerate() {
            assert_eq!(
                update_collection
                    .get_protocol_parameters_for_ledger_index(index)
                    .await
                    .unwrap()
                    .unwrap()
                    .parameters
                    .version,
                version as u8
            );
        }

        let mut parameters = ProtocolParameters::from(iota_types::block::protocol::protocol_parameters());
        parameters.version = 10;
        parameters.token_supply = u64::MAX;

        update_collection
            .upsert_protocol_parameters(1500.into(), parameters)
            .await
            .unwrap();

        assert_eq!(
            update_collection.get_latest_protocol_parameters().await.unwrap(),
            update_collection.get_protocol_parameters_for_version(10).await.unwrap()
        );

        teardown(db).await;
    }
}
