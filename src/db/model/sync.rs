// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use futures::stream::Stream;
use mongodb::{bson::doc, error::Error, options::FindOptions};
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

impl MongoDb {
    /// Retrieves the sync records sorted by [`milestone_index`](SyncRecord::milestone_index).
    pub async fn sync_records_sorted(&self) -> Result<impl Stream<Item = Result<SyncRecord, Error>>, Error> {
        self.0
            .collection::<SyncRecord>(collection::SYNC_RECORDS)
            .find(
                doc! { "synced": true },
                FindOptions::builder().sort(doc! {"milestone_index": 1u32}).build(),
            )
            .await
    }
}
