// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{IndexOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{collections::milestone::MilestoneDocument, MongoDb},
    types::{
        ledger::{BlockMetadata, LedgerInclusionState, OutputMetadata, OutputWithMetadata, SpentMetadata},
        stardust::block::{Block, BlockId, Output, OutputId, TransactionId},
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
    /// The index of this block in white flag order.
    white_flag_index: u32,
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
                    doc! { "$replaceRoot": { "newRoot": "$block" } },
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
        let raw = self
            .0
            .collection::<Block>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "block_id": block_id } },
                    doc! { "$replaceRoot": { "newRoot": "$raw" } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;

        Ok(raw)
    }

    /// Get the metadata of a [`Block`] by its [`BlockId`].
    pub async fn get_block_metadata(&self, block_id: &BlockId) -> Result<Option<BlockMetadata>, Error> {
        let block = self
            .0
            .collection::<Block>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "block_id": block_id } },
                    doc! { "$replaceRoot": { "newRoot": "$metadata" } },
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

    /// Inserts a [`Block`] together with its associated [`BlockMetadata`].
    pub async fn insert_block_with_metadata(
        &self,
        block_id: BlockId,
        block: Block,
        raw: Vec<u8>,
        metadata: BlockMetadata,
        white_flag_index: u32,
    ) -> Result<(), Error> {
        let block_document = BlockDocument {
            block_id,
            block,
            raw,
            metadata,
            white_flag_index,
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
                    doc! { "$replaceRoot": { "newRoot": "$block" } },
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

    /// Get an [`Output`] by [`OutputId`].
    pub async fn get_output(&self, output_id: &OutputId) -> Result<Option<Output>, Error> {
        let output = self
            .0
            .collection::<Output>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                        "block.payload.transaction_id": &output_id.transaction_id,
                        "$expr": { "$gt": [{ "$size": "$block.payload.essence.outputs" }, &(output_id.index as i64)] }
                    } },
                    doc! { "$replaceRoot": { "newRoot": { "$arrayElemAt": [ "$block.payload.essence.outputs", &(output_id.index as i64) ] } } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;

        Ok(output)
    }

    /// Get an [`OutputWithMetadata`] by [`OutputId`].
    pub async fn get_output_with_metadata(&self, output_id: &OutputId) -> Result<Option<OutputWithMetadata>, Error> {
        let mut output: Option<OutputWithMetadata> = self
            .0
            .collection::<OutputWithMetadata>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                        "block.payload.transaction_id": &output_id.transaction_id,
                        "$expr": { "$gt": [{ "$size": "$block.payload.essence.outputs" }, &(output_id.index as i64)] }
                    } },
                    doc! { "$lookup": {
                        "from": MilestoneDocument::COLLECTION,
                        "localField": "metadata.referenced_by_milestone_index",
                        "foreignField": "milestone_index",
                        "as": "metadata.referenced_by_milestone"
                    } },
                    doc! { "$unwind": "$metadata.referenced_by_milestone" },
                    doc! { "$replaceRoot": { "newRoot": {
                        "output": { "$arrayElemAt": [ "$block.payload.essence.outputs", &(output_id.index as i64) ] } ,
                        "metadata": {
                            "output_id": &output_id,
                            "block_id": "$block_id",
                            "transaction_id": "$block.payload.transaction_id",
                            "booked": {
                                "milestone_index": "$metadata.referenced_by_milestone_index",
                                "milestone_timestamp": "$metadata.referenced_by_milestone.milestone_timestamp",
                            },
                        }
                    } } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;
        let spent_metadata = self.get_spending_transaction_metadata(output_id).await?;
        if let Some(output) = output.as_mut() {
            output.metadata.spent = spent_metadata;
        }

        Ok(output)
    }

    /// Get an [`OutputWithMetadata`] by [`OutputId`].
    pub async fn get_output_metadata(&self, output_id: &OutputId) -> Result<Option<OutputMetadata>, Error> {
        let mut metadata: Option<OutputMetadata> = self
            .0
            .collection::<OutputMetadata>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                        "block.payload.transaction_id": &output_id.transaction_id,
                        "$expr": { "$gt": [{ "$size": "$block.payload.essence.outputs" }, &(output_id.index as i64)] }
                    } },
                    doc! { "$lookup": {
                        "from": MilestoneDocument::COLLECTION,
                        "localField": "metadata.referenced_by_milestone_index",
                        "foreignField": "milestone_index",
                        "as": "metadata.referenced_by_milestone"
                    } },
                    doc! { "$unwind": "$metadata.referenced_by_milestone" },
                    doc! { "$replaceRoot": { "newRoot": {
                        "output_id": &output_id,
                        "block_id": "$block_id",
                        "transaction_id": "$block.payload.transaction_id",
                        "booked": {
                            "milestone_index": "$metadata.referenced_by_milestone_index",
                            "milestone_timestamp": "$metadata.referenced_by_milestone.milestone_timestamp",
                        },
                    } } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;
        let spent_metadata = self.get_spending_transaction_metadata(output_id).await?;
        if let Some(metadata) = metadata.as_mut() {
            metadata.spent = spent_metadata;
        }

        Ok(metadata)
    }

    /// Gets the spending transaction of an [`Output`] by [`OutputId`].
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
                    doc! { "$replaceRoot": { "newRoot": "$block" } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?)
    }

    /// Gets the spending transaction metadata of an [`Output`] by [`OutputId`].
    pub async fn get_spending_transaction_metadata(
        &self,
        output_id: &OutputId,
    ) -> Result<Option<SpentMetadata>, Error> {
        let metadata = self
            .0
            .collection::<SpentMetadata>(BlockDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                        "block.payload.essence.inputs.transaction_id": &output_id.transaction_id,
                        "block.payload.essence.inputs.index": &(output_id.index as i32),
                    } },
                    doc! { "$lookup": {
                        "from": MilestoneDocument::COLLECTION,
                        "localField": "metadata.referenced_by_milestone_index",
                        "foreignField": "milestone_index",
                        "as": "metadata.referenced_by_milestone"
                    } },
                    doc! { "$unwind": "$metadata.referenced_by_milestone" },
                    doc! { "$replaceRoot": { "newRoot": {
                        "transaction_id": "$block.payload.transaction_id",
                        "spent": {
                            "milestone_index": "$metadata.referenced_by_milestone_index",
                            "milestone_timestamp": "$metadata.referenced_by_milestone.milestone_timestamp",
                        },
                    } } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;
        Ok(metadata)
    }
}
