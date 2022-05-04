// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use futures::stream::Stream;
use mongodb::{
    bson::{self, doc, Document},
    error::Error,
    options::{FindOptions, UpdateOptions},
    results::UpdateResult,
};
use serde::{Deserialize, Serialize};

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

impl SyncRecord {
    /// The status collection name.
    pub const COLLECTION: &'static str = "sync";
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

impl MongoDb {
    /// Get milestone with index.
    pub async fn get_sync_record_by_index(&self, index: u32) -> Result<Option<Document>, Error> {
        let res = self
            .0
            .collection::<Document>(SyncRecord::COLLECTION)
            .find_one(doc! {"milestone_index": index}, None)
            .await;

        Ok(res.unwrap()) // Fix the `DocErr` type
    }

    /// Upserts a [`SyncRecord`] to the database.
    pub async fn upsert_sync_record(&self, record: &SyncRecord) -> Result<UpdateResult, Error> {
        self.0
            .collection::<SyncRecord>(SyncRecord::COLLECTION)
            .update_one(
                doc! {"milestone_index": record.milestone_index},
                doc! {"$set": bson::to_document(record)?},
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
    }
    /// Retrieves the sync records sorted by [`milestone_index`](SyncRecord::milestone_index).
    pub async fn sync_records_sorted(&self) -> Result<impl Stream<Item = Result<SyncRecord, Error>>, Error> {
        self.0
            .collection::<SyncRecord>(SyncRecord::COLLECTION)
            .find(
                doc! { "synced": true },
                FindOptions::builder().sort(doc! {"milestone_index": 1u32}).build(),
            )
            .await
    }
}
