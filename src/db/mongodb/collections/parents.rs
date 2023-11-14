// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{prelude::stream::TryStreamExt, Stream};
use iota_sdk::types::block::{Block, BlockId};
use mongodb::{
    bson::doc,
    options::{IndexOptions, InsertManyOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    db::{
        mongodb::{DbError, InsertIgnoreDuplicatesExt},
        MongoDb, MongoDbCollection, MongoDbCollectionExt,
    },
    model::{block_metadata::BlockWithMetadata, SerializeToBson},
};

/// Chronicle Parents record which relates child to parent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParentsDocument {
    /// The parent id.
    parent_id: BlockId,
    /// The child id.
    child_id: BlockId,
}

/// The iota block parents collection.
pub struct ParentsCollection {
    collection: mongodb::Collection<ParentsDocument>,
}

#[async_trait::async_trait]
impl MongoDbCollection for ParentsCollection {
    const NAME: &'static str = "iota_parents";
    type Document = ParentsDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), DbError> {
        self.create_index(
            IndexModel::builder()
                .keys(doc! { "parent_id": 1, "child_id": 1 })
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .name("parent_child_index".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}

impl ParentsCollection {
    /// Inserts [`SignedBlock`]s together with their associated [`BlockMetadata`].
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_blocks<'a, I>(&self, blocks_with_metadata: I) -> Result<(), DbError>
    where
        I: IntoIterator<Item = &'a BlockWithMetadata>,
        I::IntoIter: Send + Sync,
    {
        let docs = blocks_with_metadata.into_iter().flat_map(|b| {
            match b.block.inner().block() {
                Block::Basic(b) => b.strong_parents().into_iter(),
                Block::Validation(b) => b.strong_parents().into_iter(),
            }
            .map(|parent_id| ParentsDocument {
                parent_id: *parent_id,
                child_id: b.metadata.block_id,
            })
        });

        self.insert_many_ignore_duplicates(docs, InsertManyOptions::builder().ordered(false).build())
            .await?;

        Ok(())
    }

    /// Get the children of a block as a stream of [`BlockId`]s.
    pub async fn get_block_children(
        &self,
        block_id: &BlockId,
        page_size: usize,
        page: usize,
    ) -> Result<impl Stream<Item = Result<BlockId, DbError>>, DbError> {
        #[derive(Deserialize)]
        struct Res {
            child_id: BlockId,
        }

        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "parent_id": block_id.to_bson() } },
                    doc! { "$limit": page_size as i64 },
                    doc! { "$skip": page as i64 },
                    doc! { "$project": {
                        "child_id": 1,
                    } },
                ],
                None,
            )
            .await?
            .map_err(Into::into)
            .map_ok(|Res { child_id }| child_id))
    }
}
