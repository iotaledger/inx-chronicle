// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::stream::Stream;
use mongodb::{
    bson::{self, doc},
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

impl MongoDb {
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
