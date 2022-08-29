// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, StreamExt, TryStreamExt};
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{IndexOptions, InsertManyOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::{PayloadKind, INSERT_BATCH_SIZE};
use crate::{
    db::MongoDb,
    types::{
        ledger::{BlockMetadata, LedgerInclusionState},
        stardust::block::{Block, BlockId, OutputId, Payload, TransactionId},
        tangle::MilestoneIndex,
    },
};

/// Chronicle Block record.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BlockDocument {
    #[serde(rename = "_id")]
    block_id: BlockId,
    /// The block.
    block: Block,
    /// The raw bytes of the block.
    #[serde(with = "serde_bytes")]
    raw: Vec<u8>,
    /// The block's metadata.
    metadata: BlockMetadata,
}

impl BlockDocument {
    /// The stardust blocks collection name.
    const COLLECTION: &'static str = "stardust_blocks";
}

/// Implements the queries for the core API.
impl MongoDb {
    /// Creates block indexes.
    pub async fn create_block_indexes(&self) -> Result<(), Error> {
        let collection = self.db.collection::<BlockDocument>(BlockDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "block.payload.transaction_id": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .name("transaction_id_index".to_string())
                            .partial_filter_expression(doc! {
                                "block.payload.transaction_id": { "$exists": true },
                                "metadata.inclusion_state": { "$eq": LedgerInclusionState::Included },
                            })
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        Ok(())
    }

    /// Get a [`Block`] by its [`BlockId`].
    pub async fn get_block(&self, block_id: &BlockId) -> Result<Option<Block>, Error> {
        let block = self
            .db
            .collection::<Block>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "_id": block_id } },
                    doc! { "$replaceWith": "$block" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;

        Ok(block)
    }

    /// Get the raw bytes of a [`Block`] by its [`BlockId`].
    pub async fn get_block_raw(&self, block_id: &BlockId) -> Result<Option<Vec<u8>>, Error> {
        #[derive(Deserialize)]
        struct RawResult {
            #[serde(with = "serde_bytes")]
            data: Vec<u8>,
        }

        let raw = self
            .db
            .collection::<RawResult>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "_id": block_id } },
                    doc! { "$replaceWith": { "data": "$raw" } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document::<RawResult>)
            .transpose()?;

        Ok(raw.map(|i| i.data))
    }

    /// Get the metadata of a [`Block`] by its [`BlockId`].
    pub async fn get_block_metadata(&self, block_id: &BlockId) -> Result<Option<BlockMetadata>, Error> {
        let block = self
            .db
            .collection::<BlockMetadata>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "_id": block_id } },
                    doc! { "$replaceWith": "$metadata" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;

        Ok(block)
    }

    /// Get the children of a [`Block`] as a stream of [`BlockId`]s.
    pub async fn get_block_children(
        &self,
        block_id: &BlockId,
        page_size: usize,
        page: usize,
    ) -> Result<impl Stream<Item = Result<BlockId, Error>>, Error> {
        #[derive(Deserialize)]
        struct BlockIdResult {
            block_id: BlockId,
        }

        Ok(self
            .db
            .collection::<BlockIdResult>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "block.parents": block_id } },
                    doc! { "$skip": (page_size * page) as i64 },
                    doc! { "$sort": {"metadata.referenced_by_milestone_index": -1} },
                    doc! { "$limit": page_size as i64 },
                    doc! { "$replaceWith": { "block_id": "$metadata.block_id" } },
                ],
                None,
            )
            .await?
            .map(|doc| Ok(bson::from_document::<BlockIdResult>(doc?)?.block_id)))
    }

    /// Inserts a [`Block`] together with its associated [`BlockMetadata`].
    pub async fn insert_block_with_metadata(
        &self,
        block_id: BlockId,
        block: Block,
        raw: Vec<u8>,
        metadata: BlockMetadata,
    ) -> Result<(), Error> {
        if metadata.inclusion_state == LedgerInclusionState::Included {
            if let Some(Payload::TreasuryTransaction(payload)) = &block.payload {
                self.insert_treasury(metadata.referenced_by_milestone_index, payload.as_ref())
                    .await?;
            }
        }

        self.db
            .collection::<BlockDocument>(BlockDocument::COLLECTION)
            .insert_one(
                BlockDocument {
                    block_id,
                    block,
                    raw,
                    metadata,
                },
                None,
            )
            .await?;

        Ok(())
    }

    /// Inserts [`Block`]s together with their associated [`BlockMetadata`].
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_blocks_with_metadata(
        &self,
        blocks_with_metadata: impl IntoIterator<Item = (BlockId, Block, Vec<u8>, BlockMetadata)>,
    ) -> Result<(), Error> {
        let blocks_with_metadata = blocks_with_metadata
            .into_iter()
            .map(|(block_id, block, raw, metadata)| BlockDocument {
                block_id,
                block,
                raw,
                metadata,
            })
            .collect::<Vec<_>>();
        self.insert_treasury_payloads(blocks_with_metadata.iter().filter_map(|block_document| {
            if block_document.metadata.inclusion_state == LedgerInclusionState::Included {
                if let Some(Payload::TreasuryTransaction(payload)) = &block_document.block.payload {
                    return Some((block_document.metadata.referenced_by_milestone_index, payload.as_ref()));
                }
            }
            None
        }))
        .await?;

        for batch in blocks_with_metadata.chunks(INSERT_BATCH_SIZE) {
            self.collection::<BlockDocument>(BlockDocument::COLLECTION)
                .insert_many_ignore_duplicates(batch, InsertManyOptions::builder().ordered(false).build())
                .await?;
        }

        Ok(())
    }

    /// Finds the [`Block`] that included a transaction by [`TransactionId`].
    pub async fn get_block_for_transaction(&self, transaction_id: &TransactionId) -> Result<Option<Block>, Error> {
        let block = self
            .db
            .collection::<Block>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                        "block.payload.transaction_id": transaction_id,
                    } },
                    doc! { "$replaceWith": "$block" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;

        Ok(block)
    }

    /// Gets the spending transaction of an [`Output`](crate::types::stardust::block::Output) by [`OutputId`].
    pub async fn get_spending_transaction(&self, output_id: &OutputId) -> Result<Option<Block>, Error> {
        Ok(self
            .db
            .collection::<Block>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                        "block.payload.essence.inputs.transaction_id": &output_id.transaction_id,
                        "block.payload.essence.inputs.index": &(output_id.index as i32)
                    } },
                    doc! { "$replaceWith": "$block" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?)
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct BlockAnalyticsResult {
    pub count: u64,
}

impl MongoDb {
    /// Gathers block analytics.
    pub async fn get_block_analytics<B: PayloadKind>(
        &self,
        start_index: Option<MilestoneIndex>,
        end_index: Option<MilestoneIndex>,
    ) -> Result<BlockAnalyticsResult, Error> {
        let mut queries = vec![doc! {
            "$nor": [
                { "metadata.referenced_by_milestone_index": { "$lt": start_index } },
                { "metadata.referenced_by_milestone_index": { "$gte": end_index } },
            ],
        }];
        if let Some(kind) = B::kind() {
            queries.push(doc! { "block.payload.kind": kind });
        }
        Ok(self
            .db
            .collection::<BlockAnalyticsResult>(BlockDocument::COLLECTION)
            .aggregate(
                vec![doc! { "$match": { "$and": queries } }, doc! { "$count": "count" }],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?
            .unwrap_or_default())
    }
}

#[cfg(all(test, feature = "test-db"))]
mod test_db {
    use bee_block_stardust as bee;
    use packable::PackableExt;

    use crate::{
        db::collections::test::connect_to_test_db,
        types::{
            ledger::{BlockMetadata, ConflictReason, LedgerInclusionState},
            stardust::block::{
                tests::{get_test_milestone_block, get_test_tagged_data_block, get_test_transaction_block},
                TransactionPayload,
            },
        },
    };

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

        db.insert_blocks_with_metadata(blocks.clone())
            .await
            .unwrap();

        for (block_id, block, _, _) in blocks.iter() {
            assert_eq!(db.get_block(block_id).await.unwrap().as_ref(), Some(block));
        }

        for (block_id, _, raw, _) in blocks.iter() {
            assert_eq!(
                db.get_block_raw(block_id).await.unwrap().as_ref(),
                Some(raw),
            );
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
}
