mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use std::collections::HashMap;

    use chronicle::{
        db::{collections::TreasuryCollection, MongoDbCollectionExt},
        types::{
            stardust::block::payload::{MilestoneId, TreasuryTransactionPayload},
            tangle::MilestoneIndex,
        },
    };
    use iota_types::block::rand::number::rand_number_range;

    use super::common::{setup_collection, setup_database, teardown};

    #[tokio::test]
    async fn test_insert_treasury_updates() {
        let db = setup_database("test-insert-treasury-updates").await.unwrap();
        let update_collection = setup_collection::<TreasuryCollection>(&db).await.unwrap();

        let ctx = iota_types::block::protocol::protocol_parameters();
        let mut milestones = HashMap::new();

        for (milestone_index, payload) in
            (0..10u32).map(|milestone_index| (milestone_index, TreasuryTransactionPayload::rand(&ctx)))
        {
            milestones.insert(milestone_index, payload.input_milestone_id);

            update_collection
                .insert_treasury(milestone_index.into(), &payload)
                .await
                .unwrap();
        }

        assert_eq!(update_collection.count().await.unwrap(), 10);
        assert_eq!(
            &update_collection
                .get_latest_treasury()
                .await
                .unwrap()
                .unwrap()
                .milestone_id,
            milestones.get(&9).unwrap()
        );

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_insert_many_treasury_updates() {
        let db = setup_database("test-insert-many-treasury-updates").await.unwrap();
        let update_collection = setup_collection::<TreasuryCollection>(&db).await.unwrap();

        let mut milestones = HashMap::new();

        let treasury_updates = (0..10u32)
            .map(|milestone_index| {
                (
                    MilestoneIndex::from(milestone_index),
                    MilestoneId::rand(),
                    rand_number_range(1000..10000000u64),
                )
            })
            .inspect(|(milestone_index, milestone_id, _)| {
                milestones.insert(milestone_index.0, *milestone_id);
            })
            .collect::<Vec<_>>();

        update_collection
            .insert_treasury_payloads(treasury_updates)
            .await
            .unwrap();

        assert_eq!(update_collection.count().await.unwrap(), 10);
        assert_eq!(
            &update_collection
                .get_latest_treasury()
                .await
                .unwrap()
                .unwrap()
                .milestone_id,
            milestones.get(&9).unwrap()
        );

        teardown(db).await;
    }
}
