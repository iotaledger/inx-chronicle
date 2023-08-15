// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod common;

#[cfg(feature = "rand")]
mod test_rand {
    use std::{collections::HashSet, fs::File, io::BufReader};

    use chronicle::{
        db::{mongodb::collections::BlockCollection, MongoDbCollectionExt},
        model::{
            metadata::{BlockMetadata, ConflictReason, LedgerInclusionState},
            payload::Payload,
            utxo::OutputId,
            Block, BlockId,
        },
    };
    use futures::TryStreamExt;
    use packable::PackableExt;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct BlockTestData {
        #[serde(rename = "_id")]
        block_id: BlockId,
        #[serde(with = "serde_bytes")]
        raw: Vec<u8>,
        metadata: BlockMetadata,
    }

    use super::common::{setup_collection, setup_database, teardown};

    #[tokio::test]
    async fn test_blocks() {
        let db = setup_database("test-blocks").await.unwrap();
        let block_collection = setup_collection::<BlockCollection>(&db).await.unwrap();
        let file = File::open("tests/data/blocks_ms_2418807.json").unwrap();
        let test_data: mongodb::bson::Bson = serde_json::from_reader(BufReader::new(file)).unwrap();

        let blocks: Vec<BlockTestData> = mongodb::bson::from_bson(test_data).unwrap();

        let blocks = blocks
            .into_iter()
            .map(
                |BlockTestData {
                     block_id,
                     raw,
                     metadata,
                 }| {
                    let block = iota_sdk::types::block::Block::unpack_unverified(raw.clone())
                        .unwrap()
                        .into();
                    (block_id, block, raw, metadata)
                },
            )
            .collect::<Vec<_>>();

        block_collection
            .insert_blocks_with_metadata(blocks.clone())
            .await
            .unwrap();

        for (transaction_id, block) in blocks.iter().filter_map(|(_, block, _, _)| {
            block.payload.as_ref().and_then(|p| {
                if let Payload::Transaction(payload) = p {
                    Some((payload.transaction_id, block))
                } else {
                    None
                }
            })
        }) {
            assert_eq!(
                block_collection
                    .get_block_for_transaction(&transaction_id)
                    .await
                    .unwrap()
                    .map(|res| res.block)
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
                iota_sdk::types::block::rand::bytes::rand_bytes(100),
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

        let mut s = block_collection
            .get_block_children(&parents[0], 1.into(), 15, 100, 0)
            .await
            .unwrap();

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

        // The spent block in the sample data is at white flag index 44.
        let file = File::open("tests/data/blocks_ms_2418187.json").unwrap();
        let test_data: mongodb::bson::Bson = serde_json::from_reader(BufReader::new(file)).unwrap();

        let blocks: Vec<BlockTestData> = mongodb::bson::from_bson(test_data).unwrap();

        let spent_block_id = blocks[44].block_id;

        let blocks = blocks
            .into_iter()
            .map(
                |BlockTestData {
                     block_id,
                     raw,
                     metadata,
                 }| {
                    let block = iota_sdk::types::block::Block::unpack_unverified(raw.clone())
                        .unwrap()
                        .into();
                    (block_id, block, raw, metadata)
                },
            )
            .collect::<Vec<_>>();

        block_collection.insert_blocks_with_metadata(blocks).await.unwrap();

        // The spending block in the sample data is at white flag index 66.
        let file = File::open("tests/data/blocks_ms_2418807.json").unwrap();
        let test_data: mongodb::bson::Bson = serde_json::from_reader(BufReader::new(file)).unwrap();

        let blocks: Vec<BlockTestData> = mongodb::bson::from_bson(test_data).unwrap();

        let blocks = blocks
            .into_iter()
            .map(
                |BlockTestData {
                     block_id,
                     raw,
                     metadata,
                 }| {
                    let block: Block = iota_sdk::types::block::Block::unpack_unverified(raw.clone())
                        .unwrap()
                        .into();
                    (block_id, block, raw, metadata)
                },
            )
            .collect::<Vec<_>>();

        let spending_block = blocks[66].1.clone();

        block_collection
            .insert_blocks_with_metadata(blocks.clone())
            .await
            .unwrap();

        let spent_block = block_collection.get_block(&spent_block_id).await.unwrap().unwrap();

        let spent_output_id = if let Some(Payload::Transaction(payload)) = &spent_block.payload {
            OutputId::from((payload.transaction_id, 0))
        } else {
            unreachable!()
        };

        assert_eq!(
            spending_block,
            block_collection
                .get_spending_transaction(&spent_output_id)
                .await
                .unwrap()
                .unwrap()
        );

        teardown(db).await;
    }

    #[tokio::test]
    async fn test_pastcone_whiteflag_order() {
        let db = setup_database("test-pastcone-whiteflag-order").await.unwrap();
        let block_collection = setup_collection::<BlockCollection>(&db).await.unwrap();

        let mut block_ids = Vec::with_capacity(10);

        let mut blocks = std::iter::repeat_with(BlockId::rand)
            .take(10)
            .inspect(|block_id| block_ids.push(*block_id))
            .enumerate()
            .map(|(i, block_id)| {
                let block = Block::rand_no_payload();
                let parents = block.parents.clone();
                (
                    block_id,
                    block,
                    iota_sdk::types::block::rand::bytes::rand_bytes(100),
                    BlockMetadata {
                        parents,
                        is_solid: true,
                        should_promote: false,
                        should_reattach: false,
                        referenced_by_milestone_index: 1.into(),
                        milestone_index: 0.into(),
                        inclusion_state: LedgerInclusionState::NoTransaction,
                        conflict_reason: ConflictReason::None,
                        white_flag_index: i as u32,
                    },
                )
            })
            .collect::<Vec<_>>();

        // not necessary: just to make sure the insertion order is different from the white flag order.
        blocks.reverse();

        block_collection
            .insert_blocks_with_metadata(blocks.clone())
            .await
            .unwrap();

        let whiteflag_cone = block_collection
            .get_referenced_blocks_in_white_flag_order(1.into())
            .await
            .unwrap();

        assert_eq!(whiteflag_cone, block_ids);

        teardown(db).await;
    }
}
