// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_test::rand::message::rand_message;
use chronicle::{
    db::{model::stardust::message::MessageRecord, MongoDb, MongoDbConfig},
    types::{ledger::{ConflictReason, LedgerInclusionState, Metadata}, stardust::message::Message},
};
use packable::PackableExt;

#[tokio::test]
async fn test_test() -> Result<(), mongodb::error::Error> {
    let bee_message = rand_message();
    let raw = bee_message.pack_to_vec();
    let message: Message = bee_message.clone()
    .into();

    let message_id = message.message_id.clone();

    let metadata = Metadata {
        is_solid: true,
        should_promote: true,
        should_reattach: true,
        referenced_by_milestone_index: 42,
        milestone_index: 0,
        inclusion_state: LedgerInclusionState::Included,
        conflict_reason: ConflictReason::None,
    };

    let record = MessageRecord {
        inner: message,
        raw,
        metadata: Some(metadata),
    };

    let config = MongoDbConfig::default().with_suffix("cargo-test");
    let db = MongoDb::connect(&config).await?;

    db.upsert_message_record(&record).await?;

    let result = db.get_message(&message_id).await?.unwrap();
    let bee_result: bee_message_stardust::Message = result.inner.try_into().unwrap();
    assert_eq!(bee_result, bee_message);

    Ok(())
}
