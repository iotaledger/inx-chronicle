// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

use bee_block_stardust as bee;
use chronicle::{
    db::collections::{BlockCollection, OutputCollection},
    types::{
        ledger::{BlockMetadata, ConflictReason, LedgerInclusionState, LedgerOutput, MilestoneIndexTimestamp},
        stardust::{
            block::{
                output::OutputId,
                payload::{TransactionEssence, TransactionPayload},
            },
            util::*,
        },
    },
};
use packable::PackableExt;

use crate::common::connect_to_test_db;

#[tokio::test]
async fn test_blocks() {
    let db = connect_to_test_db("test-blocks").await.unwrap();
    db.clear().await.unwrap();
    let collection = db.collection::<BlockCollection>();
    collection.create_indexes().await.unwrap();

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

    collection.insert_blocks_with_metadata(blocks.clone()).await.unwrap();

    // Without the outputs inserted separately, this block is not complete.
    assert_ne!(
        collection.get_block(&blocks[0].0).await.unwrap().as_ref(),
        Some(&blocks[0].1)
    );

    let transaction_payload = TransactionPayload::try_from(blocks[0].1.clone().payload.unwrap()).unwrap();
    let TransactionEssence::Regular { outputs, .. } = transaction_payload.essence;

    db.collection::<OutputCollection>()
        .insert_unspent_outputs(
            Vec::from(outputs)
                .into_iter()
                .enumerate()
                .map(|(i, output)| LedgerOutput {
                    output_id: OutputId {
                        transaction_id: transaction_payload.transaction_id,
                        index: i as u16,
                    },
                    block_id: blocks[0].0,
                    booked: MilestoneIndexTimestamp {
                        milestone_index: 0.into(),
                        milestone_timestamp: 12345.into(),
                    },
                    output,
                }),
        )
        .await
        .unwrap();

    for (block_id, block, _, _) in &blocks {
        assert_eq!(collection.get_block(block_id).await.unwrap().as_ref(), Some(block));
    }

    for (block_id, _, raw, _) in &blocks {
        assert_eq!(collection.get_block_raw(block_id).await.unwrap().as_ref(), Some(raw),);
    }

    for (block_id, _, _, metadata) in &blocks {
        assert_eq!(
            collection.get_block_metadata(block_id).await.unwrap().as_ref(),
            Some(metadata),
        );
    }

    assert_eq!(
        collection
            .get_block_for_transaction(
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
