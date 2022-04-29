// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use futures::stream::Stream;
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{FindOptions, UpdateOptions},
    results::UpdateResult,
};
use serde::{Deserialize, Serialize};

use super::collection;
use crate::db::MongoDb;

/// A record indicating that a milestone is completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    /// The index of the milestone that was completed.
    pub milestone_index: u32,
    /// Whether the milestone has been written to an archive file.
    pub logged: bool,
    /// Whether the milestone has been synced.
    pub synced: bool,
}

/// An aggregation type that represents the ranges of completed milestones and gaps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncData {
    /// The completed(synced and logged) milestones data
    pub completed: Vec<Range<u32>>,
    /// Synced milestones data but unlogged
    pub synced_but_unlogged: Vec<Range<u32>>,
    /// Gaps/missings milestones data
    pub gaps: Vec<Range<u32>>,
}

/// Implementations for [`SyncRecord`].
impl MongoDb {
    /// Upserts a [`SyncRecord`].
    pub async fn upsert_sync_record(&self, sync_record: SyncRecord) -> Result<UpdateResult, Error> {
        self.0
            .collection::<SyncRecord>(collection::SYNC_RECORDS)
            .update_one(
                doc! { "milestone_index": sync_record.milestone_index },
                doc! {"$set": bson::to_document(&sync_record)?},
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
    }

    /// Retrieves the [`SyncRecord`]s sorted by [`milestone_index`](SyncRecord::milestone_index).
    pub async fn get_sync_records_sorted(&self) -> Result<impl Stream<Item = Result<SyncRecord, Error>>, Error> {
        self.0
            .collection::<SyncRecord>(collection::SYNC_RECORDS)
            .find(
                doc! { "synced": true },
                FindOptions::builder().sort(doc! {"milestone_index": 1u32}).build(),
            )
            .await
    }
}
