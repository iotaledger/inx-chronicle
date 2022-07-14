// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{FindOptions, IndexOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        ledger::{BlockMetadata, LedgerInclusionState},
        stardust::block::{Block, BlockId, OutputId, Payload, TransactionId},
    },
};

/// Chronicle Block record.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BlockDocument {
    /// The id of the current block.
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
        let collection = self.0.collection::<BlockDocument>(BlockDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "block_id": 1 })
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
                                "block.payload.transaction_id": { "$exists": true } ,
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
            .0
            .collection::<Block>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "block_id": block_id } },
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
            .0
            .collection::<RawResult>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "block_id": block_id } },
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
            .0
            .collection::<BlockMetadata>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "block_id": block_id } },
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
        self.0
            .collection::<BlockId>(BlockDocument::COLLECTION)
            .find(
                doc! {"block.parents": bson::to_bson(block_id)?},
                FindOptions::builder()
                    .skip((page_size * page) as u64)
                    .sort(doc! {"metadata.referenced_by_milestone_index": -1})
                    .limit(page_size as i64)
                    .projection(doc! {"block_id": 1 })
                    .build(),
            )
            .await
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

        let block_document = BlockDocument {
            block_id,
            block,
            raw,
            metadata,
        };

        let mut doc = bson::to_document(&block_document)?;
        doc.insert("_id", block_id.to_hex());

        self.0
            .collection::<BlockDocument>(BlockDocument::COLLECTION)
            .update_one(
                doc! { "block_id": block_id },
                doc! { "$set": doc },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    /// Finds the [`Block`] that included a transaction by [`TransactionId`].
    pub async fn get_block_for_transaction(&self, transaction_id: &TransactionId) -> Result<Option<Block>, Error> {
        let block = self
            .0
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
            .0
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

#[cfg(feature = "api")]
mod analytics {
    use super::*;
    use crate::types::tangle::MilestoneIndex;

    #[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
    pub struct TransactionAnalyticsResult {
        pub count: u64,
        pub total_value: f64,
        pub avg_value: f64,
    }

    impl MongoDb {
        /// Gathers transaction analytics.
        pub async fn get_transaction_analytics(
            &self,
            start_milestone: MilestoneIndex,
            end_milestone: MilestoneIndex,
        ) -> Result<TransactionAnalyticsResult, Error> {
            Ok(self
                .0
                .collection::<TransactionAnalyticsResult>(BlockDocument::COLLECTION)
                .aggregate(
                    vec![
                        doc! { "$match": {
                           "block.payload.kind": "transaction",
                           "metadata.referenced_by_milestone_index": { "$gte": start_milestone, "$lte": end_milestone },
                        } },
                        doc! { "$unwind": "$block.payload.essence.outputs" },
                        doc! { "$group": {
                            "_id": "null",
                            "count": { "$sum": 1 },
                            "total_value": { "$sum": { "$toDouble": "$block.payload.essence.outputs.amount" } },
                            "avg_value": { "$avg": { "$toDouble": "$block.payload.essence.outputs.amount" } },
                        } },
                    ],
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
}
