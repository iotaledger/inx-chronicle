// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeInclusive;

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::doc,
    error::Error,
    options::{FindOneOptions, FindOptions, IndexOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::SortOrder;
use crate::{
    db::{
        mongodb::{MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::block::payload::milestone::{
        MilestoneId, MilestoneIndex, MilestoneIndexTimestamp, MilestoneOption, MilestonePayload, MilestoneTimestamp,
    },
};

const BY_OLDEST: i32 = 1;
const BY_NEWEST: i32 = -1;

/// A milestone's metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MilestoneDocument {
    /// The [`MilestoneId`](MilestoneId) of the milestone.
    #[serde(rename = "_id")]
    milestone_id: MilestoneId,
    /// The milestone index and timestamp.
    at: MilestoneIndexTimestamp,
    /// The milestone's payload.
    payload: MilestonePayload,
}

/// The stardust milestones collection.
pub struct MilestoneCollection {
    collection: mongodb::Collection<MilestoneDocument>,
}

#[async_trait::async_trait]
impl MongoDbCollection for MilestoneCollection {
    const NAME: &'static str = "stardust_milestones";
    type Document = MilestoneDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), Error> {
        self.create_index(
            IndexModel::builder()
                .keys(doc! { "at.milestone_index": BY_OLDEST })
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .name("milestone_idx_index".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "at.milestone_timestamp": BY_OLDEST })
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .name("milestone_timestamp_index".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}

/// An aggregation type that represents the ranges of completed milestones and gaps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncData {
    /// The completed(synced and logged) milestones data
    pub completed: Vec<RangeInclusive<MilestoneIndex>>,
    /// Gaps/missings milestones data
    pub gaps: Vec<RangeInclusive<MilestoneIndex>>,
}

impl MilestoneCollection {
    /// Gets the [`MilestonePayload`] of a milestone.
    pub async fn get_milestone_payload_by_id(
        &self,
        milestone_id: &MilestoneId,
    ) -> Result<Option<MilestonePayload>, Error> {
        self.aggregate(
            [
                doc! { "$match": { "_id": milestone_id } },
                doc! { "$replaceWith": "$payload" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Gets [`MilestonePayload`] of a milestone by the [`MilestoneIndex`].
    pub async fn get_milestone_payload(&self, index: MilestoneIndex) -> Result<Option<MilestonePayload>, Error> {
        self.aggregate(
            [
                doc! { "$match": { "at.milestone_index": index } },
                doc! { "$replaceWith": "$payload" },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Gets Milestone by the [`MilestoneIndex`].
    pub async fn get_milestone(
        &self,
        index: MilestoneIndex,
    ) -> Result<Option<(MilestoneId, MilestoneIndexTimestamp, MilestonePayload)>, Error> {
        self.aggregate::<MilestoneDocument>(vec![doc! { "$match": { "at.milestone_index": index } }], None)
            .await?
            .map_ok(
                |MilestoneDocument {
                     milestone_id,
                     at,
                     payload,
                 }| (milestone_id, at, payload),
            )
            .try_next()
            .await
    }

    /// Gets the [`MilestoneTimestamp`] of a milestone by [`MilestoneIndex`].
    pub async fn get_milestone_timestamp(&self, index: MilestoneIndex) -> Result<Option<MilestoneTimestamp>, Error> {
        #[derive(Deserialize)]
        struct MilestoneTimestampResult {
            milestone_timestamp: MilestoneTimestamp,
        }

        Ok(self
            .aggregate::<MilestoneTimestampResult>(
                vec![
                    doc! { "$match": { "at.milestone_index": index } },
                    doc! { "$project": {
                        "milestone_timestamp": "$at.milestone_timestamp"
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|ts| ts.milestone_timestamp))
    }

    /// Gets the id of a milestone by the [`MilestoneIndex`].
    pub async fn get_milestone_id(&self, index: MilestoneIndex) -> Result<Option<MilestoneId>, Error> {
        #[derive(Deserialize)]
        struct MilestoneIdResult {
            milestone_id: MilestoneId,
        }
        Ok(self
            .find_one::<MilestoneIdResult>(
                doc! { "at.milestone_index": index },
                FindOneOptions::builder()
                    .projection(doc! {
                        "milestone_id": "$_id",
                    })
                    .build(),
            )
            .await?
            .map(|ts| ts.milestone_id))
    }

    /// Inserts the information of a milestone into the database.
    #[instrument(skip(self, milestone_id, milestone_timestamp, payload), err, level = "trace")]
    pub async fn insert_milestone(
        &self,
        milestone_id: MilestoneId,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
        payload: MilestonePayload,
    ) -> Result<(), Error> {
        let milestone_document = MilestoneDocument {
            at: MilestoneIndexTimestamp {
                milestone_index,
                milestone_timestamp,
            },
            milestone_id,
            payload,
        };

        self.insert_one(milestone_document, None).await?;

        Ok(())
    }

    /// Find the starting milestone.
    pub async fn find_first_milestone(
        &self,
        start_timestamp: MilestoneTimestamp,
    ) -> Result<Option<MilestoneIndexTimestamp>, Error> {
        self.find(
            doc! {
                "at.milestone_timestamp": { "$gte": start_timestamp },
            },
            FindOptions::builder()
                .sort(doc! { "at.milestone_index": 1 })
                .limit(1)
                .projection(doc! {
                    "milestone_index": "$at.milestone_index",
                    "milestone_timestamp": "$at.milestone_timestamp",
                })
                .build(),
        )
        .await?
        .try_next()
        .await
    }

    /// Find the end milestone.
    pub async fn find_last_milestone(
        &self,
        end_timestamp: MilestoneTimestamp,
    ) -> Result<Option<MilestoneIndexTimestamp>, Error> {
        self.find(
            doc! {
                "at.milestone_timestamp": { "$lte": end_timestamp },
            },
            FindOptions::builder()
                .sort(doc! { "at.milestone_index": -1 })
                .limit(1)
                .projection(doc! {
                    "milestone_index": "$at.milestone_index",
                    "milestone_timestamp": "$at.milestone_timestamp",
                })
                .build(),
        )
        .await?
        .try_next()
        .await
    }

    async fn get_first_milestone_sorted(&self, order: i32) -> Result<Option<MilestoneIndexTimestamp>, Error> {
        self.aggregate(
            [
                doc! { "$sort": { "at.milestone_index": order } },
                doc! { "$limit": 1 },
                doc! { "$project": {
                    "milestone_index": "$at.milestone_index",
                    "milestone_timestamp": "$at.milestone_timestamp"
                } },
            ],
            None,
        )
        .await?
        .try_next()
        .await
    }

    /// Find the newest milestone.
    pub async fn get_newest_milestone(&self) -> Result<Option<MilestoneIndexTimestamp>, Error> {
        self.get_first_milestone_sorted(BY_NEWEST).await
    }

    /// Find the oldest milestone.
    pub async fn get_oldest_milestone(&self) -> Result<Option<MilestoneIndexTimestamp>, Error> {
        self.get_first_milestone_sorted(BY_OLDEST).await
    }

    /// Gets the current ledger index.
    pub async fn get_ledger_index(&self) -> Result<Option<MilestoneIndex>, Error> {
        Ok(self.get_newest_milestone().await?.map(|ts| ts.milestone_index))
    }

    /// Streams all available receipt milestone options together with their corresponding `MilestoneIndex`.
    pub async fn get_all_receipts(
        &self,
    ) -> Result<impl Stream<Item = Result<(MilestoneOption, MilestoneIndex), Error>>, Error> {
        #[derive(Deserialize)]
        struct ReceiptAtIndex {
            receipt: MilestoneOption,
            index: MilestoneIndex,
        }

        Ok(self
            .aggregate::<ReceiptAtIndex>(
                vec![
                    doc! { "$unwind": "$payload.essence.options"},
                    doc! { "$match": {
                        "payload.essence.options.receipt.migrated_at": { "$exists": true },
                    } },
                    doc! { "$sort": { "at.milestone_index": 1 } },
                    doc! { "$replaceWith": {
                        "receipt": "options.receipt" ,
                        "index": "$at.milestone_index" ,
                    } },
                ],
                None,
            )
            .await?
            .map_ok(|ReceiptAtIndex { receipt, index }| (receipt, index)))
    }

    /// Streams all available receipt milestone options together with their corresponding `MilestoneIndex` that were
    /// migrated at the given index.
    pub async fn get_receipts_migrated_at(
        &self,
        migrated_at: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<(MilestoneOption, MilestoneIndex), Error>>, Error> {
        #[derive(Deserialize)]
        struct ReceiptAtIndex {
            receipt: MilestoneOption,
            index: MilestoneIndex,
        }

        Ok(self
            .aggregate([
                    doc! { "$unwind": "$payload.essence.options"},
                    doc! { "$match": {
                        "payload.essence.options.receipt.migrated_at": { "$and": [ { "$exists": true }, { "$eq": migrated_at } ] },
                    } },
                    doc! { "$sort": { "at.milestone_index": 1 } },
                    doc! { "$replaceWith": {
                        "receipt": "options.receipt" ,
                        "index": "$at.milestone_index" ,
                    } },
                ],
                None,
            )
            .await?
            .map_ok(|ReceiptAtIndex { receipt, index }| (receipt, index)))
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct MilestoneResult {
    pub milestone_id: MilestoneId,
    pub index: MilestoneIndex,
}

impl MilestoneCollection {
    /// Get milestones matching given conditions.
    pub async fn get_milestones(
        &self,
        start_timestamp: Option<MilestoneTimestamp>,
        end_timestamp: Option<MilestoneTimestamp>,
        order: SortOrder,
        page_size: usize,
        cursor: Option<MilestoneIndex>,
    ) -> Result<impl Stream<Item = Result<MilestoneResult, Error>>, Error> {
        let (sort, cmp) = match order {
            SortOrder::Newest => (doc! { "at.milestone_index": -1 }, "$gt"),
            SortOrder::Oldest => (doc! { "at.milestone_index": 1 }, "$lt"),
        };

        self.aggregate(
            [
                doc! { "$match": {
                    "$nor": [
                        { "at.milestone_timestamp": { "$lt": start_timestamp } },
                        { "at.milestone_timestamp": { "$gt": end_timestamp } },
                        { "at.milestone_index": { cmp: cursor } }
                    ]
                } },
                doc! { "$sort": sort },
                doc! { "$limit": page_size as i64 },
                doc! { "$project": {
                    "milestone_id": "$_id",
                    "index": "$at.milestone_index"
                } },
            ],
            None,
        )
        .await
    }
}
