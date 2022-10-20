// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use std::collections::HashSet;

    use chronicle::{
        db::{
            collections::{BlockCollection, OutputCollection},
            MongoDbCollectionExt,
        },
        types::{
            ledger::{
                BlockMetadata, ConflictReason, LedgerInclusionState, LedgerOutput, MilestoneIndexTimestamp,
                RentStructureBytes,
            },
            stardust::block::{output::OutputId, payload::TransactionEssence, Block, BlockId, Input, Payload},
        },
    };
    use futures::TryStreamExt;

    use super::common::{setup_collection, setup_database, teardown};

    #[tokio::test]
    async fn test_blocks() {
        let db = setup_database("test-blocks").await.unwrap();
        let block_collection = setup_collection::<BlockCollection>(&db).await.unwrap();

        let protocol_params = iota_types::block::protocol::protocol_parameters();

        let blocks = std::iter::repeat_with(|| (BlockId::rand(), Block::rand(&protocol_params)))
            .take(100)
            .enumerate()
            .map(|(i, (block_id, block))| {
                let parents = block.parents.clone();
                (
                    block_id,
                    block,
                    iota_types::block::rand::bytes::rand_bytes(100),
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

        block_collection
            .insert_blocks_with_metadata(blocks.clone())
            .await
            .unwrap();

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
                block_collection
                    .get_block_for_transaction(&transaction_id)
                    .await
                    .unwrap()
                    .as_ref(),
                Some(block),
            );
        }

        for (block_id, block, _, _) in &blocks {
            assert_eq!(
                block_collection.get_block(block_id).await.unwrap().as_ref(),
                Some(block)
            );
        }

        for (block_id, _, raw, _) in &blocks {
            assert_eq!(
                block_collection.get_block_raw(block_id).await.unwrap().as_ref(),
                Some(raw)
            );
        }

        for (block_id, _, _, metadata) in &blocks {
            assert_eq!(
                block_collection.get_block_metadata(block_id).await.unwrap().as_ref(),
                Some(metadata),
            );
        }
        teardown(db).await;
    }

    #[tokio::test]
    async fn test_block_children() {
        let db = setup_database("test-children").await.unwrap();
        let block_collection = setup_collection::<BlockCollection>(&db).await.unwrap();

        let parents = std::iter::repeat_with(BlockId::rand)
            .take(2)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut children = HashSet::new();

        let f = |(i, (block_id, block)): (usize, (BlockId, Block))| {
            let parents = block.parents.clone();
            (
                block_id,
                block,
                iota_types::block::rand::bytes::rand_bytes(100),
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
                children.insert(*block_id);
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

        block_collection
            .insert_blocks_with_metadata(blocks.clone())
            .await
            .unwrap();
        assert_eq!(block_collection.count().await.unwrap(), 10);

        let mut s = block_collection.get_block_children(&parents[0], 100, 0).await.unwrap();

        while let Some(child_id) = s.try_next().await.unwrap() {
            assert!(children.remove(&child_id))
        }
        assert!(children.is_empty());

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_spending_transaction() {
        let db = setup_database("test-spending-transaction").await.unwrap();
        let block_collection = setup_collection::<BlockCollection>(&db).await.unwrap();
        let output_collection = setup_collection::<OutputCollection>(&db).await.unwrap();

        let ctx = iota_types::block::protocol::protocol_parameters();

        let (block_id, block, transaction_id, input_id, outputs) =
            std::iter::repeat_with(|| (BlockId::rand(), Block::rand(&ctx)))
                .filter_map(|(block_id, block)| {
                    block.payload.as_ref().and_then(|p| {
                        if let Payload::Transaction(payload) = p {
                            let TransactionEssence::Regular { inputs, outputs, .. } = &payload.essence;
                            for input in inputs.iter() {
                                if let Input::Utxo(input_id) = input {
                                    let input_id = *input_id;
                                    let outputs = outputs.to_vec();
                                    if !outputs.is_empty() {
                                        return Some((
                                            block_id,
                                            block.clone(),
                                            payload.transaction_id,
                                            input_id,
                                            outputs,
                                        ));
                                    }
                                }
                            }
                        }
                        None
                    })
                })
                .take(1)
                .collect::<Vec<_>>()
                .pop()
                .unwrap();

        let parents = block.parents.clone();
        let raw = iota_types::block::rand::bytes::rand_bytes(100);
        let metadata = BlockMetadata {
            parents,
            is_solid: true,
            should_promote: false,
            should_reattach: false,
            referenced_by_milestone_index: 2.into(),
            milestone_index: 1.into(),
            inclusion_state: LedgerInclusionState::Included,
            conflict_reason: ConflictReason::None,
            white_flag_index: 0u32,
        };

        block_collection
            .insert_blocks_with_metadata(vec![(block_id, block.clone(), raw, metadata)])
            .await
            .unwrap();
        output_collection
            .insert_unspent_outputs(outputs.into_iter().enumerate().map(|(i, output)| LedgerOutput {
                output_id: OutputId {
                    transaction_id,
                    index: i as u16,
                },
                block_id,
                booked: MilestoneIndexTimestamp {
                    milestone_index: 1.into(),
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

        let spending_block = block_collection
            .get_spending_transaction(&input_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(spending_block, block);

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_milestone_activity() {
        let db = setup_database("test-milestone-activity").await.unwrap();
        let block_collection = setup_collection::<BlockCollection>(&db).await.unwrap();

        let protocol_params = iota_types::block::protocol::protocol_parameters();

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
                iota_types::block::rand::bytes::rand_bytes(100),
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

        block_collection
            .insert_blocks_with_metadata(blocks.clone())
            .await
            .unwrap();

        let activity = block_collection
            .get_milestone_activity_analytics(1.into())
            .await
            .unwrap();

        assert_eq!(activity.count, 5);
        assert_eq!(activity.transaction_count, 1);
        assert_eq!(activity.treasury_transaction_count, 1);
        assert_eq!(activity.milestone_count, 1);
        assert_eq!(activity.tagged_data_count, 1);
        assert_eq!(activity.no_payload_count, 1);
        assert_eq!(activity.confirmed_count, 1);
        assert_eq!(activity.conflicting_count, 1);
        assert_eq!(activity.no_transaction_count, 3);

        teardown(db).await;
    }
}
