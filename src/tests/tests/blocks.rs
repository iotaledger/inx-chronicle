// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust as bee;
use chronicle::types::{
    ledger::{BlockMetadata, ConflictReason, LedgerInclusionState},
    stardust::{block::payload::TransactionPayload, util::*},
};
use inx_chronicle_tests::connect_to_test_db;
use packable::PackableExt;

#[tokio::test]
async fn test_blocks() {
    let db = connect_to_test_db().await.unwrap().database("test-blocks");
    db.clear().await.unwrap();
    db.create_block_indexes().await.unwrap();

    let blocks = vec![
        get_test_transaction_block(),
        get_test_milestone_block(),
        get_test_tagged_data_block(),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, block)| {
        let bee_block = bee::Block::try_from(block.clone()).unwrap();
        let parents = block.parents.clone();
        (
            bee_block.id().into(),
            block,
            bee_block.pack_to_vec(),
            BlockMetadata {
                parents,
                is_solid: true,
                should_promote: false,
                should_reattach: false,
                referenced_by_milestone_index: 1.into(),
                milestone_index: 0.into(),
                inclusion_state: LedgerInclusionState::Included,
                conflict_reason: ConflictReason::None,
                white_flag_index: i as u32,
            },
        )
    })
    .collect::<Vec<_>>();

    db.insert_blocks_with_metadata(blocks.clone()).await.unwrap();

    for (block_id, block, _, _) in blocks.iter() {
        assert_eq!(db.get_block(block_id).await.unwrap().as_ref(), Some(block));
    }

    for (block_id, _, raw, _) in blocks.iter() {
        assert_eq!(db.get_block_raw(block_id).await.unwrap().as_ref(), Some(raw),);
    }

    assert_eq!(
        db.get_block_for_transaction(
            &TransactionPayload::try_from(blocks[0].1.clone().payload.unwrap())
                .unwrap()
                .transaction_id
        )
        .await
        .unwrap()
        .as_ref(),
        Some(&blocks[0].1),
    );

    db.drop().await.unwrap();
}
