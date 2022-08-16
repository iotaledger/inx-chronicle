// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, StreamExt, TryStreamExt};
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::IndexOptions,
    ClientSession, IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::PayloadKind;
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
                    .keys(doc! { "metadata.block_id": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .name("block_id_index".to_string())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

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
                    doc! { "$match": { "metadata.block_id": block_id } },
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
                    doc! { "$match": { "metadata.block_id": block_id } },
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
                    doc! { "$match": { "metadata.block_id": block_id } },
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
        let block_id = metadata.block_id;
        let block_document = BlockDocument { block, raw, metadata };

        let mut doc = bson::to_document(&block_document)?;
        doc.insert("_id", block_id.to_hex());

        self.db
            .collection::<bson::Document>(BlockDocument::COLLECTION)
            .insert_one(doc, None)
            .await?;

        Ok(())
    }

    /// Inserts [`Block`]s together with their associated [`BlockMetadata`].
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_blocks_with_metadata(
        &self,
        session: &mut ClientSession,
        blocks_with_metadata: impl IntoIterator<Item = (Block, Vec<u8>, BlockMetadata)>,
    ) -> Result<(), Error> {
        let blocks_with_metadata = blocks_with_metadata
            .into_iter()
            .map(|(block, raw, metadata)| BlockDocument { block, raw, metadata })
            .collect::<Vec<_>>();
        if !blocks_with_metadata.is_empty() {
            self.insert_treasury_payloads(
                session,
                blocks_with_metadata.iter().filter_map(|block_document| {
                    if block_document.metadata.inclusion_state == LedgerInclusionState::Included {
                        if let Some(Payload::TreasuryTransaction(payload)) = &block_document.block.payload {
                            return Some((block_document.metadata.referenced_by_milestone_index, payload.as_ref()));
                        }
                    }
                    None
                }),
            )
            .await?;
            let blocks_with_metadata = blocks_with_metadata
                .into_iter()
                .map(|block_document| {
                    let block_id = block_document.metadata.block_id;
                    let mut doc = bson::to_document(&block_document)?;
                    doc.insert("_id", block_id.to_hex());
                    Result::<_, Error>::Ok(doc)
                })
                .collect::<Result<Vec<_>, _>>()?;

            self.db
                .collection::<bson::Document>(BlockDocument::COLLECTION)
                .insert_many_with_session(blocks_with_metadata, None, session)
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
