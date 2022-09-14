// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

use chronicle::{
    db::collections::{BlockCollection, OutputCollection},
    types::{
        ledger::{BlockMetadata, ConflictReason, LedgerInclusionState, LedgerOutput, MilestoneIndexTimestamp},
        stardust::block::{output::OutputId, payload::TransactionEssence, Block, BlockId, Payload},
    },
};

use crate::common::connect_to_test_db;

#[tokio::test]
#[cfg(feature = "rand")]
async fn test_blocks() {
    let db = connect_to_test_db("test-blocks").await.unwrap();
    db.clear().await.unwrap();
    let collection = db.collection::<BlockCollection>();
    collection.create_indexes().await.unwrap();

    let blocks = std::iter::repeat_with(|| (BlockId::rand(), Block::rand()))
        .take(100)
        .enumerate()
        .map(|(i, (block_id, block))| {
            let parents = block.parents.clone();
            (
                block_id,
                block,
                bee_block_stardust::rand::bytes::rand_bytes(100),
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

    for (block_id, transaction_id, block, outputs) in blocks.iter().filter_map(|(block_id, block, _, _)| {
        block.payload.as_ref().and_then(|p| {
            if let Payload::Transaction(payload) = p {
                let TransactionEssence::Regular { outputs, .. } = &payload.essence;
                Some((block_id, payload.transaction_id, block, outputs))
            } else {
                None
            }
        })
    }) {
        if !outputs.is_empty() {
            db.collection::<OutputCollection>()
                .insert_unspent_outputs(Vec::from(outputs.clone()).into_iter().enumerate().map(|(i, output)| {
                    LedgerOutput {
                        output_id: OutputId {
                            transaction_id,
                            index: i as u16,
                        },
                        block_id: *block_id,
                        booked: MilestoneIndexTimestamp {
                            milestone_index: 0.into(),
                            milestone_timestamp: 12345.into(),
                        },
                        output,
                    }
                }))
                .await
                .unwrap();
        }

        assert_eq!(
            collection
                .get_block_for_transaction(&transaction_id)
                .await
                .unwrap()
                .as_ref(),
            Some(block),
        );
    }

    for (block_id, block, _, _) in &blocks {
        assert_eq!(collection.get_block(block_id).await.unwrap().as_ref(), Some(block));
    }

    for (block_id, _, raw, _) in &blocks {
        assert_eq!(collection.get_block_raw(block_id).await.unwrap().as_ref(), Some(raw));
    }

    for (block_id, _, _, metadata) in &blocks {
        assert_eq!(
            collection.get_block_metadata(block_id).await.unwrap().as_ref(),
            Some(metadata),
        );
    }

    db.drop().await.unwrap();
}
