mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use chronicle::{
        db::collections::MilestoneCollection,
        types::stardust::block::payload::{MilestoneId, MilestonePayload},
    };

    use super::common::{setup_collection, setup_database, teardown};

    #[tokio::test]
    async fn test_milestones() {
        let db = setup_database("test-milestones").await.unwrap();
        let milestone_collection = setup_collection::<MilestoneCollection>(&db).await.unwrap();

        let milestone = MilestonePayload::rand(&iota_types::block::protocol::protocol_parameters());
        let milestone_id = MilestoneId::rand();

        milestone_collection
            .insert_milestone(
                milestone_id,
                milestone.essence.index,
                milestone.essence.timestamp.into(),
                milestone.clone(),
            )
            .await
            .unwrap();

        assert_eq!(
            milestone_collection
                .get_milestone_id(milestone.essence.index)
                .await
                .unwrap(),
            Some(milestone_id),
        );

        assert_eq!(
            milestone_collection
                .get_milestone_payload_by_id(&milestone_id)
                .await
                .unwrap()
                .as_ref(),
            Some(&milestone)
        );

        assert_eq!(
            milestone_collection
                .get_milestone_payload(milestone.essence.index)
                .await
                .unwrap()
                .as_ref(),
            Some(&milestone)
        );

        teardown(db).await;
    }
}
