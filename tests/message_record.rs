// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_test::rand::block::rand_block;
use chronicle::db::{
    model::{
        ledger::{ConflictReason, LedgerInclusionState, Metadata},
        stardust::block::{Block, BlockRecord},
    },
    MongoDb, MongoDbConfig,
};
use packable::PackableExt;

#[ignore]
#[tokio::test]
async fn test_test() -> Result<(), mongodb::error::Error> {
    let bee_block = rand_block();
    let raw = bee_block.pack_to_vec();
    let block: Block = bee_block.clone().into();

    let block_id = block.block_id.clone();

    let metadata = Metadata {
        is_solid: true,
        should_promote: true,
        should_reattach: true,
        referenced_by_milestone_index: 42.into(),
        milestone_index: 0.into(),
        inclusion_state: LedgerInclusionState::Included,
        conflict_reason: ConflictReason::None,
    };

    let record = BlockRecord {
        inner: block,
        raw,
        metadata: Some(metadata),
    };

    let config = MongoDbConfig::default().with_suffix("cargo-test");
    let db = MongoDb::connect(&config).await?;

    db.clear().await?;

    db.upsert_block_record(&record).await?;

    let result = db.get_block(&block_id).await?.unwrap();
    let bee_result: bee_block_stardust::Block = result.inner.try_into().unwrap();
    assert_eq!(bee_result, bee_block);

    Ok(())
}
