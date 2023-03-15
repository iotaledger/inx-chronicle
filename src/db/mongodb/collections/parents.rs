// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::doc,
    error::Error,
    options::{IndexOptions, InsertManyOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        mongodb::{InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::{tangle::MilestoneIndex, BlockId},
};

/// Chronicle Parent Child Relationship record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParentsDocument {
    pub(crate) parent_id: BlockId,
    pub(crate) child_id: BlockId,
    pub(crate) milestone_index: MilestoneIndex,
}

/// The stardust blocks collection.
pub struct ParentsCollection {
    collection: mongodb::Collection<ParentsDocument>,
}

#[async_trait::async_trait]
impl MongoDbCollection for ParentsCollection {
    const NAME: &'static str = "stardust_parents";
    type Document = ParentsDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), Error> {
        self.create_index(
            IndexModel::builder()
                .keys(doc! { "parent_id": 1, "child_id": 1, "milestone_index": 1 })
                .options(
                    IndexOptions::builder()
                        .name("block_parents_index".to_string())
                        .unique(true)
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
    /// Insert parent/child relationships, given the referenced milestone indexes of the children.
    pub async fn insert_relationships<I, B>(&self, docs: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = B>,
        I::IntoIter: Send + Sync,
        ParentsDocument: From<B>,
    {
        let docs = docs.into_iter().map(ParentsDocument::from);

        self.insert_many_ignore_duplicates(docs, InsertManyOptions::builder().ordered(false).build())
            .await?;

        Ok(())
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
                    doc! { "$match": { "parent_id": parent_id } },
                    doc! { "$skip": (page_size * page) as i64 },
                    doc! { "$sort": { "milestone_index": -1 } },
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
