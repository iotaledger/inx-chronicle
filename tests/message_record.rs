// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_test::rand::block::rand_block;
use chronicle::{
    db::{MongoDb, MongoDbConfig},
    types::{
        ledger::{BlockMetadata, ConflictReason, LedgerInclusionState},
        stardust::block::Block,
    },
};
use packable::PackableExt;

#[ignore]
#[tokio::test]
async fn test_test() -> Result<(), mongodb::error::Error> {
    let bee_block = rand_block();
    let raw = bee_block.pack_to_vec();
    let block: Block = bee_block.clone().into();

    let block_id = block.block_id.clone();

    let metadata = BlockMetadata {
        is_solid: true,
        block_id: block_id.clone(),
        parents: block.parents.clone(),
        should_promote: true,
        should_reattach: true,
        referenced_by_milestone_index: 42.into(),
        milestone_index: 0.into(),
        inclusion_state: LedgerInclusionState::Included,
        conflict_reason: ConflictReason::None,
    };

    let config = MongoDbConfig::default().with_suffix("cargo-test");
    let db = MongoDb::connect(&config).await?;

    db.clear().await?;

    db.insert_block_with_metadata(block_id.clone(), block, raw, metadata)
        .await?;

    let result = db.get_block(&block_id).await?.unwrap();
    let bee_result: bee_block_stardust::Block = result.try_into().unwrap();
    assert_eq!(bee_result, bee_block);

    Ok(())
}
