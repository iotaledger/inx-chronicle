// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeInclusive;

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{FindOneOptions, FindOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        stardust::{
            block::{MilestoneId, MilestonePayload, Payload},
            milestone::MilestoneTimestamp,
        },
        tangle::MilestoneIndex,
    },
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct SyncStatus {
    /// Indicates if all blocks of a milestone were successfully synchronized.
    has_all_blocks: bool,
    /// Indicates if all ledger updates of a milestone were successfully synchronized.
    has_all_ledger_updates: bool,
}

/// A milestone's metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct MilestoneDocument {
    /// The milestone index.
    #[serde(rename = "_id")]
    milestone_index: MilestoneIndex,
    /// The [`MilestoneId`](MilestoneId) of the milestone.
    milestone_id: MilestoneId,
    /// The timestamp of the milestone.
    milestone_timestamp: MilestoneTimestamp,
    /// The milestone's payload.
    payload: MilestonePayload,
    /// The milestone's sync status.
    sync_status: SyncStatus,
}

impl MilestoneDocument {
    /// The stardust milestone collection name.
    const COLLECTION: &'static str = "stardust_milestones";
}

/// An aggregation type that represents the ranges of completed milestones and gaps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncData {
    /// The completed(synced and logged) milestones data
    pub completed: Vec<RangeInclusive<MilestoneIndex>>,
    /// Gaps/missings milestones data
    pub gaps: Vec<RangeInclusive<MilestoneIndex>>,
}

impl MongoDb {
    /// Get the [`Payload`] of a milestone.
    pub async fn get_milestone_payload_by_id(&self, milestone_id: &MilestoneId) -> Result<Option<Payload>, Error> {
        self.0
            .collection::<Payload>(MilestoneDocument::COLLECTION)
            .find_one(
                doc! {"milestone_id": bson::to_bson(milestone_id)?},
                FindOneOptions::builder().projection(doc! {"payload": 1 }).build(),
            )
            .await
    }

    /// Get [`Payload`] of a milestone by the [`MilestoneIndex`].
    pub async fn get_milestone_payload(&self, index: MilestoneIndex) -> Result<Option<Payload>, Error> {
        self.0
            .collection::<Payload>(MilestoneDocument::COLLECTION)
            .find_one(
                doc! {"_id": index},
                FindOneOptions::builder().projection(doc! {"payload": 1 }).build(),
            )
            .await
    }

    /// Inserts the information of a milestone into the database.
    pub async fn insert_milestone(
        &self,
        milestone_id: MilestoneId,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
        payload: MilestonePayload,
    ) -> Result<(), Error> {
        let milestone_document = MilestoneDocument {
            milestone_id,
            milestone_index,
            milestone_timestamp,
            payload,
            sync_status: Default::default(),
        };

        self.0
            .collection::<MilestoneDocument>(MilestoneDocument::COLLECTION)
            .insert_one(milestone_document, None)
            .await?;

        Ok(())
    }

    /// Find the starting milestone.
    pub async fn find_first_milestone(
        &self,
        start_timestamp: MilestoneTimestamp,
    ) -> Result<Option<MilestoneIndex>, Error> {
        Ok(self
            .0
            .collection::<MilestoneDocument>(MilestoneDocument::COLLECTION)
            .find(
                doc! {"milestone_timestamp": { "$gte": start_timestamp }},
                FindOptions::builder()
                    .sort(doc! {"milestone_index": 1})
                    .limit(1)
                    .build(),
            )
            .await?
            .try_next()
            .await?
            .map(|d| d.milestone_index))
    }

    /// Find the end milestone.
    pub async fn find_last_milestone(
        &self,
        end_timestamp: MilestoneTimestamp,
    ) -> Result<Option<MilestoneIndex>, Error> {
        Ok(self
            .0
            .collection::<MilestoneDocument>(MilestoneDocument::COLLECTION)
            .find(
                doc! {"milestone_timestamp": { "$lte": end_timestamp }},
                FindOptions::builder()
                    .sort(doc! {"milestone_index": -1})
                    .limit(1)
                    .build(),
            )
            .await?
            .try_next()
            .await?
            .map(|d| d.milestone_index))
    }

    /// If a milestone is available, returns if of its [`Block`](crate::types::stardust::block::Block)s have been
    /// synchronized.
    pub async fn get_sync_status_blocks(&self, index: MilestoneIndex) -> Result<Option<bool>, Error> {
        self.0
            .collection::<bool>(MilestoneDocument::COLLECTION)
            .find_one(
                doc! {"_id": index},
                FindOneOptions::builder()
                    .projection(doc! {"sync_status.has_all_blocks": 1 })
                    .build(),
            )
            .await
    }

    /// Marks that all [`Block`](crate::types::stardust::block::Block)s of a milestone have been synchronized.
    pub async fn set_sync_status_blocks(&self, index: MilestoneIndex) -> Result<(), Error> {
        self.0
            .collection::<MilestoneDocument>(MilestoneDocument::COLLECTION)
            .update_one(
                doc! { "_id": index },
                doc! { "$set": {
                    "sync_status.has_all_blocks": true,

                }},
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    /// Retrieves the sync records sorted by [`milestone_index`](SyncRecord::milestone_index).
    async fn get_sorted_milestone_indices_synced(
        &self,
        range: RangeInclusive<MilestoneIndex>,
    ) -> Result<impl Stream<Item = Result<MilestoneIndex, Error>>, Error> {
        #[derive(Deserialize)]
        struct SyncEntry {
            #[serde(rename = "_id")]
            milestone_index: MilestoneIndex,
        }

        self.0
            .collection::<SyncEntry>(MilestoneDocument::COLLECTION)
            .find(
                doc! {
                    "_id": { "$gte": *range.start(), "$lte": *range.end() },
                    "sync_status.has_all_blocks": { "$eq": true }
                },
                FindOptions::builder()
                    .sort(doc! {"_id": 1u32})
                    .projection(doc! {"_id": 1u32})
                    .build(),
            )
            .await
            .map(|c| c.map_ok(|e| e.milestone_index))
    }

    /// Retrieves a [`SyncData`] structure that contains the completed and gaps ranges.
    pub async fn get_sync_data(&self, range: RangeInclusive<MilestoneIndex>) -> Result<SyncData, Error> {
        let mut synced_ms = self.get_sorted_milestone_indices_synced(range.clone()).await?;
        let mut sync_data = SyncData::default();
        let mut last_record: Option<MilestoneIndex> = None;

        while let Some(milestone_index) = synced_ms.try_next().await? {
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
                    if *last.end() + 1 == milestone_index {
                        *last = *last.start()..=milestone_index;
                    } else {
                        sync_data.completed.push(milestone_index..=milestone_index);
                    }
                }
                None => sync_data.completed.push(milestone_index..=milestone_index),
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
