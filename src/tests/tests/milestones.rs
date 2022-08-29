// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust as bee;
use chronicle::types::stardust::{block::payload::MilestoneId, util::payload::milestone::get_test_milestone_payload};
use inx_chronicle_tests::connect_to_test_db;

#[tokio::test]
async fn test_milestones() {
    let db = connect_to_test_db().await.unwrap().database("test-milestones");
    db.clear().await.unwrap();
    db.create_milestone_indexes().await.unwrap();

    let milestone = get_test_milestone_payload();
    let milestone_id = MilestoneId::from(
        bee::payload::MilestonePayload::try_from(milestone.clone())
            .unwrap()
            .id(),
    );

    db.insert_milestone(
        milestone_id,
        milestone.essence.index,
        milestone.essence.timestamp.into(),
        milestone.clone(),
    )
    .await
    .unwrap();

    assert_eq!(
        db.get_milestone_id(milestone.essence.index).await.unwrap(),
        Some(milestone_id),
    );

    assert_eq!(
        db.get_milestone_payload_by_id(&milestone_id).await.unwrap().as_ref(),
        Some(&milestone)
    );

    assert_eq!(
        db.get_milestone_payload(milestone.essence.index)
            .await
            .unwrap()
            .as_ref(),
        Some(&milestone)
    );

    db.drop().await.unwrap();
}
