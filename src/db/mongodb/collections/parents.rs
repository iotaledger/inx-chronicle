// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
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
    pub(crate) parent_referenced_index: MilestoneIndex,
    pub(crate) child_id: BlockId,
    pub(crate) child_referenced_index: MilestoneIndex,
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
                .keys(doc! { "parent_referenced_index": 1 })
                .options(
                    IndexOptions::builder()
                        .name("parent_referenced_index".to_string())
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
    /// 
    /// Note, that for `solidified index` it must be true that: `solidified_index < current_ledger_index - below_max_depth`, or this method
    /// may not return all of a parents children.
    pub async fn get_solidified_relationships(
        &self,
        solidified_index: MilestoneIndex,
    ) -> Result<Vec<(BlockId, Vec<BlockId>)>, Error> {
        #[derive(Deserialize)]
        struct ParentChildrenResult {
            #[serde(rename = "_id")]
            parent_id: BlockId,
            children: Vec<BlockId>,
        }
        Ok(self
            .aggregate::<ParentChildrenResult>(
                [
                    doc! { "$match": {
                        "parent_referenced_index": { "$eq": solidified_index },
                    } },
                    doc! { "$group": {
                        "_id": "$parent_id",
                        "children": { "$push": "$child_id" },
                    } },
                ],
                None,
            )
            .await?
            .map_ok(|ParentChildrenResult { parent_id, children }| (parent_id, children))
            .try_collect()
            .await?)
    }
}
