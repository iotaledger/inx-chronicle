// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use std::collections::HashSet;

    use chronicle::{
        db::{
            collections::{BlockCollection, OutputCollection},
            MongoDb,
        },
        types::{
            ledger::{
                BlockMetadata, ConflictReason, LedgerInclusionState, LedgerOutput, MilestoneIndexTimestamp,
                RentStructureBytes,
            },
            stardust::block::{output::OutputId, payload::TransactionEssence, Block, BlockId, Payload},
        },
    };
    use futures::TryStreamExt;

    use super::common::connect_to_test_db;

    async fn setup(database_name: impl ToString) -> (MongoDb, BlockCollection) {
        let db = connect_to_test_db(database_name).await.unwrap();
        db.clear().await.unwrap();
        let collection = db.collection::<BlockCollection>();
        collection.create_indexes().await.unwrap();
        (db, collection)
    }

    async fn teardown(db: MongoDb) {
        db.drop().await.unwrap();
    }

    #[tokio::test]
    async fn test_blocks() {
        let (db, collection) = setup("test-blocks").await;

        let protocol_params = bee_block_stardust::protocol::protocol_parameters();

        let blocks = std::iter::repeat_with(|| (BlockId::rand(), Block::rand(&protocol_params)))
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
                    .insert_unspent_outputs(outputs.iter().cloned().enumerate().map(|(i, output)| LedgerOutput {
                        output_id: OutputId {
                            transaction_id,
                            index: i as u16,
                        },
                        block_id: *block_id,
                        booked: MilestoneIndexTimestamp {
                            milestone_index: 0.into(),
                            milestone_timestamp: 12345.into(),
                        },
                        rent_structure: RentStructureBytes {
                            num_key_bytes: 0,
                            num_data_bytes: 100,
                        },
                        output,
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
        teardown(db).await;
    }

    #[tokio::test]
    async fn test_block_children() {
        let (db, collection) = setup("test-children").await;

        let parents = std::iter::repeat_with(|| BlockId::rand())
            .take(2)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut children = HashSet::new();

        let f = |(i, (block_id, block)): (usize, (BlockId, Block))| {
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
        };

        let blocks = std::iter::repeat_with(|| (BlockId::rand(), Block::rand_no_payload_with_parents(parents.clone())))
            .take(5)
            .inspect(|(block_id, _)| {
                children.insert(block_id.clone());
            })
            .enumerate()
            .map(f)
            .chain(
                std::iter::repeat_with(|| (BlockId::rand(), Block::rand_no_payload()))
                    .take(5)
                    .enumerate()
                    .map(f),
            )
            .collect::<Vec<_>>();

        collection.insert_blocks_with_metadata(blocks.clone()).await.unwrap();
        assert_eq!(collection.len().await.unwrap(), 10);

        let mut s = collection.get_block_children(&parents[0], 100, 0).await.unwrap();

        while let Some(child_id) = s.try_next().await.unwrap() {
            assert!(children.remove(&child_id))
        }
        assert!(children.is_empty());

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_milestone_activity() {
        let (db, collection) = setup("test-milestone-activity").await;

        let protocol_params = bee_block_stardust::protocol::protocol_parameters();

        let blocks = vec![
            Block::rand_treasury_transaction(&protocol_params),
            Block::rand_transaction(&protocol_params),
            Block::rand_milestone(&protocol_params),
            Block::rand_tagged_data(),
            Block::rand_no_payload(),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, block)| {
            let parents = block.parents.clone();
            (
                BlockId::rand(),
                block,
                bee_block_stardust::rand::bytes::rand_bytes(100),
                BlockMetadata {
                    parents,
                    is_solid: true,
                    should_promote: false,
                    should_reattach: false,
                    referenced_by_milestone_index: 1.into(),
                    milestone_index: 0.into(),
                    inclusion_state: match i {
                        0 => LedgerInclusionState::Included,
                        1 => LedgerInclusionState::Conflicting,
                        _ => LedgerInclusionState::NoTransaction,
                    },
                    conflict_reason: match i {
                        0 => ConflictReason::None,
                        1 => ConflictReason::InputUtxoNotFound,
                        _ => ConflictReason::None,
                    },
                    white_flag_index: i as u32,
                },
            )
        })
        .collect::<Vec<_>>();

        collection.insert_blocks_with_metadata(blocks.clone()).await.unwrap();

        let activity = collection.get_milestone_activity(1.into()).await.unwrap();

        assert_eq!(activity.num_blocks, 5);
        assert_eq!(activity.num_tx_payload, 1);
        assert_eq!(activity.num_treasury_tx_payload, 1);
        assert_eq!(activity.num_milestone_payload, 1);
        assert_eq!(activity.num_tagged_data_payload, 1);
        assert_eq!(activity.num_no_payload, 1);
        assert_eq!(activity.num_confirmed_tx, 1);
        assert_eq!(activity.num_conflicting_tx, 1);
        assert_eq!(activity.num_no_tx, 3);

        teardown(db).await;
    }
}
