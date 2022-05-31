// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use bee_test::rand::block::{rand_block, rand_block_id};
use chronicle::{
    db::MongoDbConfig,
    types::{
        ledger::{BlockMetadata, ConflictReason, LedgerInclusionState},
        stardust::block::{Block, BlockId},
    },
};
use once_cell::sync::Lazy;
use packable::PackableExt;
use tokio::sync::{Mutex, MutexGuard};

static DB_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct MongoDb<'a> {
    db: chronicle::db::MongoDb,
    _guard: MutexGuard<'a, ()>,
}

impl<'a> MongoDb<'a> {
    async fn connect(config: &MongoDbConfig) -> Result<MongoDb<'a>, mongodb::error::Error> {
        let _guard = DB_LOCK.lock().await;

        let db = chronicle::db::MongoDb::connect(config).await?;
        db.clear().await?;

        Ok(Self { db, _guard })
    }
}

impl<'a> Deref for MongoDb<'a> {
    type Target = chronicle::db::MongoDb;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

#[ignore]
#[tokio::test]
async fn insert_and_get_block() -> Result<(), mongodb::error::Error> {
    let bee_block = rand_block();
    let raw = bee_block.pack_to_vec();
    let block: Block = bee_block.clone().into();

    let block_id: BlockId = rand_block_id().into();

    let metadata = BlockMetadata {
        is_solid: true,
        block_id,
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

    db.insert_block_with_metadata(block_id, block, raw, metadata, 0).await?;

    let result_block = db.get_block(&block_id).await?.unwrap();
    let bee_result: bee_block_stardust::Block = result_block.try_into().unwrap();
    assert_eq!(bee_result, bee_block);

    Ok(())
}
