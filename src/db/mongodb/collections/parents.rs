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

/// The [`Id`] of a [`ParentsDocument`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Id {
    parent_id: BlockId,
    child_id: BlockId,
}

/// Chronicle Parent Child Relationship record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParentsDocument {
    _id: Id,
    milestone_index: MilestoneIndex,
}

pub struct ParentChildRelationship {
    pub parent_id: BlockId,
    pub child_id: BlockId,
    pub milestone_index: MilestoneIndex,
}

impl From<ParentChildRelationship> for ParentsDocument {
    fn from(value: ParentChildRelationship) -> Self {
        Self {
            _id: Id {
                parent_id: value.parent_id,
                child_id: value.child_id,
            },
            milestone_index: value.milestone_index,
        }
    }
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
                .keys(doc! { "milestone_index": 1 })
                .options(
                    IndexOptions::builder()
                        .name("block_parents_milestone_index".to_string())
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

    /// Get the children of a [`Block`] as a stream of [`BlockId`]s.
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
                    doc! { "$match": { "_id.parent_id": parent_id } },
                    doc! { "$skip": (page_size * page) as i64 },
                    doc! { "$sort": { "milestone_index": -1 } },
                    doc! { "$limit": page_size as i64 },
                    doc! { "$project": {
                        "_id": 0,
                        "child_id": "$_id.child_id"
                    } },
                ],
                None,
            )
            .await?
            .map_ok(|ChildIdResult { child_id }| child_id))
    }
}
