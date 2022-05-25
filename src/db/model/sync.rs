// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Range, RangeInclusive};

use futures::{stream::Stream, TryStreamExt};
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{FindOptions, UpdateOptions},
    results::UpdateResult,
};
use serde::{Deserialize, Serialize};

use crate::db::{model::tangle::MilestoneIndex, MongoDb};

/// A record indicating that a milestone is completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    /// The index of the milestone that was completed.
    pub milestone_index: MilestoneIndex,
}

impl SyncRecord {
    /// The status collection name.
    pub const COLLECTION: &'static str = "sync";
}

/// An aggregation type that represents the ranges of completed milestones and gaps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncData {
    /// The completed(synced and logged) milestones data
    pub completed: Vec<Range<MilestoneIndex>>,
    /// Gaps/missings milestones data
    pub gaps: Vec<RangeInclusive<MilestoneIndex>>,
}

impl MongoDb {
    /// If available, returns the [`SyncRecord`] associated with the provided milestone `index`.
    pub async fn get_sync_record_by_index(&self, index: MilestoneIndex) -> Result<Option<SyncRecord>, Error> {
        self.0
            .collection::<SyncRecord>(SyncRecord::COLLECTION)
            .find_one(doc! {"milestone_index": index}, None)
            .await
    }

    /// Upserts a [`SyncRecord`] to the database.
    pub async fn upsert_sync_record(&self, index: MilestoneIndex) -> Result<UpdateResult, Error> {
        self.0
            .collection::<SyncRecord>(SyncRecord::COLLECTION)
            .update_one(
                doc! {"_id": index},
                doc! {"$set": bson::to_document(&SyncRecord{milestone_index: index})?},
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
    }
    /// Retrieves the sync records sorted by [`milestone_index`](SyncRecord::milestone_index).
    pub async fn sync_records_sorted(
        &self,
        range: RangeInclusive<MilestoneIndex>,
    ) -> Result<impl Stream<Item = Result<SyncRecord, Error>>, Error> {
        self.0
            .collection::<SyncRecord>(SyncRecord::COLLECTION)
            .find(
                doc! { "milestone_index": { "$gte": range.start(), "$lte": range.end() } },
                FindOptions::builder().sort(doc! {"milestone_index": 1}).build(),
            )
            .await
    }

    /// Retrieves a [`SyncData`] structure that contains the completed and gaps ranges.
    pub async fn get_sync_data(&self, range: RangeInclusive<MilestoneIndex>) -> Result<SyncData, Error> {
        let mut res = self.sync_records_sorted(range.clone()).await?;
        let mut sync_data = SyncData::default();
        let mut last_record: Option<MilestoneIndex> = None;
        while let Some(SyncRecord { milestone_index }) = res.try_next().await? {
            // Missing records go into gaps
            if let Some(&last) = last_record.as_ref() {
                if last + 1 < milestone_index {
                    sync_data.gaps.push(last + 1..=milestone_index - 1);
                }
            } else if *range.start() < milestone_index {
                sync_data.gaps.push(*range.start()..=milestone_index - 1)
            }
            match sync_data.completed.last_mut() {
                Some(last) => {
                    if last.end + 1 == milestone_index {
                        last.end += 1;
                    } else {
                        sync_data.completed.push(milestone_index..milestone_index);
                    }
                }
                None => sync_data.completed.push(milestone_index..milestone_index),
            }
            last_record.replace(milestone_index);
        }
        if let Some(&last) = last_record.as_ref() {
            if last < *range.end() {
                sync_data.gaps.push(last + 1..=*range.end());
            }
        } else if range.start() <= range.end() {
            sync_data.gaps.push(range);
        }
        Ok(sync_data)
    }
}
