// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

use bee_block_stardust as bee;
use chronicle::{
    db::collections::{MilestoneCollection, MilestoneStats},
    types::stardust::{block::payload::MilestoneId, util::payload::milestone::get_test_milestone_payload},
};
use common::connect_to_test_db;

#[tokio::test]
async fn test_milestones() {
    let db = connect_to_test_db("test-milestones").await.unwrap();
    db.clear().await.unwrap();
    let collection = db.collection::<MilestoneCollection>();
    collection.create_indexes().await.unwrap();

    let milestone = get_test_milestone_payload();
    let milestone_id = MilestoneId::from(
        bee::payload::MilestonePayload::try_from(milestone.clone())
            .unwrap()
            .id(),
    );

    collection
        .insert_milestone(
            milestone_id,
            milestone.essence.index,
            milestone.essence.timestamp.into(),
            milestone.clone(),
            MilestoneStats::default(),
        )
        .await
        .unwrap();

    assert_eq!(
        collection.get_milestone_id(milestone.essence.index).await.unwrap(),
        Some(milestone_id),
    );

    assert_eq!(
        collection
            .get_milestone_payload_by_id(&milestone_id)
            .await
            .unwrap()
            .as_ref(),
        Some(&milestone)
    );

    assert_eq!(
        collection
            .get_milestone_payload(milestone.essence.index)
            .await
            .unwrap()
            .as_ref(),
        Some(&milestone)
    );

    db.drop().await.unwrap();
}
