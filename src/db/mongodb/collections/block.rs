// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::doc,
    error::Error,
    options::{IndexOptions, InsertManyOptions},
    IndexModel,
};
use packable::PackableExt;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::{parents::ParentsDocument, ParentsCollection, SortOrder};
use crate::{
    db::{
        mongodb::{InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::{
        metadata::{BlockMetadata, LedgerInclusionState},
        payload::TransactionId,
        tangle::MilestoneIndex,
        utxo::OutputId,
        Block, BlockId,
    },
    tangle::BlockData,
};

/// Chronicle Block record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockDocument {
    #[serde(rename = "_id")]
    block_id: BlockId,
    /// The block.
    block: Block,
    /// The raw bytes of the block.
    #[serde(with = "serde_bytes")]
    raw: Vec<u8>,
    /// The block's metadata.
    metadata: BlockMetadata,
    /// The block's children (updated once all children must have been arrived).
    children: Vec<BlockId>,
}

impl From<BlockData> for BlockDocument {
    fn from(
        BlockData {
            block_id,
            block,
            raw,
            metadata,
        }: BlockData,
    ) -> Self {
        Self {
            block_id,
            block,
            raw,
            metadata,
            children: Vec::new(),
        }
    }
}

impl From<(BlockId, Block, Vec<u8>, BlockMetadata)> for BlockDocument {
    fn from((block_id, block, raw, metadata): (BlockId, Block, Vec<u8>, BlockMetadata)) -> Self {
        Self {
            block_id,
            block,
            raw,
            metadata,
            children: Vec::new(),
        }
    }
}

/// The stardust blocks collection.
pub struct BlockCollection {
    collection: mongodb::Collection<BlockDocument>,
    parents_collection: ParentsCollection,
}

#[async_trait::async_trait]
impl MongoDbCollection for BlockCollection {
    const NAME: &'static str = "stardust_blocks";
    type Document = BlockDocument;

    fn instantiate(db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self {
            collection,
            parents_collection: db.collection(),
        }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), Error> {
        self.create_index(
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

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "metadata.referenced_by_milestone_index": -1, "metadata.white_flag_index": 1, "metadata.inclusion_state": 1 })
                .options(
                    IndexOptions::builder()
                        .name("block_referenced_index_comp".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct IncludedBlockResult {
    #[serde(rename = "_id")]
    pub block_id: BlockId,
    pub block: Block,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IncludedBlockMetadataResult {
    #[serde(rename = "_id")]
    pub block_id: BlockId,
    pub metadata: BlockMetadata,
}

#[derive(Deserialize)]
struct RawResult {
    #[serde(with = "serde_bytes")]
    raw: Vec<u8>,
}

#[derive(Deserialize)]
struct BlockIdResult {
    #[serde(rename = "_id")]
    block_id: BlockId,
}

/// Implements the queries for the core API.
impl BlockCollection {
    /// Get a [`Block`] by its [`BlockId`].
    pub async fn get_block(&self, block_id: &BlockId) -> Result<Option<Block>, Error> {
        Ok(self
            .get_block_raw(block_id)
            .await?
            .map(|raw| iota_types::block::Block::unpack_unverified(raw).unwrap().into()))
    }

    /// Get the raw bytes of a [`Block`] by its [`BlockId`].
    pub async fn get_block_raw(&self, block_id: &BlockId) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "_id": block_id } },
                    doc! { "$project": { "raw": 1 } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|RawResult { raw }| raw))
    }

    /// Get the metadata of a [`Block`] by its [`BlockId`].
    pub async fn get_block_metadata(&self, block_id: &BlockId) -> Result<Option<BlockMetadata>, Error> {
        self.aggregate(
            [
                doc! { "$match": { "_id": block_id } },
                doc! { "$replaceWith": "$metadata" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Get the blocks that were referenced by the specified milestone (in White-Flag order).
    pub async fn get_referenced_blocks_in_white_flag_order(
        &self,
        index: MilestoneIndex,
    ) -> Result<Vec<BlockId>, Error> {
        let block_ids = self
            .aggregate::<BlockIdResult>(
                [
                    doc! { "$match": { "metadata.referenced_by_milestone_index": index } },
                    doc! { "$sort": { "metadata.white_flag_index": 1 } },
                    doc! { "$project": { "_id": 1 } },
                ],
                None,
            )
            .await?
            .map_ok(|res| res.block_id)
            .try_collect()
            .await?;

        Ok(block_ids)
    }

    /// Get the blocks that were referenced by the specified milestone (in White-Flag order).
    pub async fn get_referenced_blocks_in_white_flag_order_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<(BlockId, Block, Vec<u8>, BlockMetadata), Error>>, Error> {
        #[derive(Debug, Deserialize)]
        struct QueryRes {
            #[serde(rename = "_id")]
            block_id: BlockId,
            #[serde(with = "serde_bytes")]
            raw: Vec<u8>,
            metadata: BlockMetadata,
        }

        Ok(self
            .aggregate::<QueryRes>(
                [
                    doc! { "$match": { "metadata.referenced_by_milestone_index": index } },
                    doc! { "$sort": { "metadata.white_flag_index": 1 } },
                ],
                None,
            )
            .await?
            .map_ok(|r| {
                (
                    r.block_id,
                    iota_types::block::Block::unpack_unverified(r.raw.clone())
                        .unwrap()
                        .into(),
                    r.raw,
                    r.metadata,
                )
            }))
    }

    /// Get the blocks that were applied by the specified milestone (in White-Flag order).
    pub async fn get_applied_blocks_in_white_flag_order(&self, index: MilestoneIndex) -> Result<Vec<BlockId>, Error> {
        let block_ids = self
            .aggregate::<BlockIdResult>(
                [
                    doc! { "$match": {
                        "metadata.referenced_by_milestone_index": index,
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                    } },
                    doc! { "$sort": { "metadata.white_flag_index": 1 } },
                    doc! { "$project": { "_id": 1 } },
                ],
                None,
            )
            .await?
            .map_ok(|res| res.block_id)
            .try_collect()
            .await?;

        Ok(block_ids)
    }

    /// Inserts [`Block`]s together with their associated [`BlockMetadata`].
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_blocks_with_metadata<I, B>(&self, blocks_with_metadata: I) -> Result<(), Error>
    where
        B: Clone,
        I: IntoIterator<Item = B>,
        I::IntoIter: Send + Sync,
        BlockDocument: From<B>,
    {
        // FIXME: unfortunately we need to collect into a Vec due to lifetime issues
        let blocks_with_metadata = blocks_with_metadata
            .into_iter()
            .map(BlockDocument::from)
            .collect::<Vec<BlockDocument>>();

        let mut parent_child_rels = Vec::with_capacity(blocks_with_metadata.len());
        for (child_id, child_metadata) in blocks_with_metadata.iter().map(|doc| (&doc.block_id, &doc.metadata)) {
            for parent_id in child_metadata.parents.iter() {
                // NOTE: we can certainly unwrap here also because it's guaranteed that we see the parents before the
                // children!
                if let Some(parent_metadata) = self.get_block_metadata(parent_id).await? {
                    parent_child_rels.push(ParentsDocument {
                        parent_id: *parent_id,
                        parent_referenced_index: parent_metadata.referenced_by_milestone_index,
                        child_id: *child_id,
                        child_referenced_index: child_metadata.referenced_by_milestone_index,
                    })
                }
            }
        }

        self.insert_many_ignore_duplicates(
            blocks_with_metadata,
            InsertManyOptions::builder().ordered(false).build(),
        )
        .await?;

        self.parents_collection.insert_relationships(parent_child_rels).await?;

        Ok(())
    }

    /// Updates the block with all of its children.
    #[instrument(skip_all, err, level = "trace")]
    pub async fn update_children(&self, parent_children_rels: Vec<(BlockId, Vec<BlockId>)>) -> Result<(), Error> {
        for (parent_id, children) in parent_children_rels {
            self.update_one(
                doc! { "_id": parent_id },
                doc! { "$set": { "children": children } },
                None,
            )
            .await?;
        }
        Ok(())
    }

    /// Finds the [`Block`] that included a transaction by [`TransactionId`].
    pub async fn get_block_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<IncludedBlockResult>, Error> {
        Ok(self.get_block_raw_for_transaction(transaction_id).await?.map(|raw| {
            let block = iota_types::block::Block::unpack_unverified(raw).unwrap();
            IncludedBlockResult {
                block_id: block.id().into(),
                block: block.into(),
            }
        }))
    }

    /// Finds the raw bytes of the block that included a transaction by [`TransactionId`].
    pub async fn get_block_raw_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .aggregate(
                [
                    doc! { "$match": {
                        "metadata.inclusion_state": LedgerInclusionState::Included,
                        "block.payload.transaction_id": transaction_id,
                    } },
                    doc! { "$project": { "raw": 1 } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|RawResult { raw }| raw))
    }

    /// Finds the [`BlockMetadata`] that included a transaction by [`TransactionId`].
    pub async fn get_block_metadata_for_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<IncludedBlockMetadataResult>, Error> {
        self.aggregate(
            [
                doc! { "$match": {
                    "metadata.inclusion_state": LedgerInclusionState::Included,
                    "block.payload.transaction_id": transaction_id,
                } },
                doc! { "$project": {
                    "_id": 1,
                    "metadata": 1,
                } },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Gets the spending transaction of an [`Output`](crate::model::utxo::Output) by [`OutputId`].
    pub async fn get_spending_transaction(&self, output_id: &OutputId) -> Result<Option<Block>, Error> {
        self.aggregate(
            [
                doc! { "$match": {
                    "metadata.inclusion_state": LedgerInclusionState::Included,
                    "block.payload.essence.inputs.transaction_id": &output_id.transaction_id,
                    "block.payload.essence.inputs.index": &(output_id.index as i32)
                } },
                doc! { "$project": { "raw": 1 } },
            ],
            None,
        )
        .await?
        .map_ok(|RawResult { raw }| iota_types::block::Block::unpack_unverified(raw).unwrap().into())
        .try_next()
        .await
    }

    /// Get the children of a [`Block`](crate::model::Block) as a stream of [`BlockId`]s.
    pub async fn get_block_children(
        &self,
        parent_id: &BlockId,
        page_size: usize,
        page: usize,
    ) -> Result<impl Stream<Item = Result<BlockId, Error>>, Error> {
        #[derive(Deserialize)]
        struct ChildIdResult {
            child_id: BlockId,
        }
        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "_id": parent_id } },
                    doc! { "$unwind": "$children" },
                    doc! { "$skip": (page_size * page) as i64 },
                    // NOTE: sorting is not supported until this functionality is show to be efficient.
                    // doc! { "$sort": { "child_milestone_index": -1 } },
                    doc! { "$limit": page_size as i64 },
                    doc! { "$project": {
                        "_id": 0,
                        "child_id": "$child_id"
                    } },
                ],
                None,
            )
            .await?
            .map_ok(|ChildIdResult { child_id }| child_id))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct BlocksByMilestoneResult {
    #[serde(rename = "_id")]
    pub block_id: BlockId,
    pub payload_kind: Option<String>,
    pub white_flag_index: u32,
}

impl BlockCollection {
    /// Get the [`Block`]s in a milestone by index as a stream of [`BlockId`]s.
    pub async fn get_blocks_by_milestone_index(
        &self,
        milestone_index: MilestoneIndex,
        page_size: usize,
        cursor: Option<u32>,
        sort: SortOrder,
    ) -> Result<impl Stream<Item = Result<BlocksByMilestoneResult, Error>>, Error> {
        let (sort, cmp) = match sort {
            SortOrder::Newest => (doc! {"metadata.white_flag_index": -1 }, "$lte"),
            SortOrder::Oldest => (doc! {"metadata.white_flag_index": 1 }, "$gte"),
        };

        let mut queries = vec![doc! { "metadata.referenced_by_milestone_index": milestone_index }];
        if let Some(white_flag_index) = cursor {
            queries.push(doc! { "metadata.white_flag_index": { cmp: white_flag_index } });
        }

        self.aggregate(
            [
                doc! { "$match": { "$and": queries } },
                doc! { "$sort": sort },
                doc! { "$limit": page_size as i64 },
                doc! { "$project": {
                    "_id": 1,
                    "payload_kind": "$block.payload.kind",
                    "white_flag_index": "$metadata.white_flag_index"
                } },
            ],
            None,
        )
        .await
    }
}
