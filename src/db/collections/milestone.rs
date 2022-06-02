// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeInclusive;

use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{FindOneOptions, FindOptions, IndexOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        stardust::{
            block::{MilestoneId, MilestonePayload},
            milestone::MilestoneTimestamp,
        },
        tangle::MilestoneIndex,
    },
};

/// A milestone's metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct MilestoneDocument {
    /// The milestone index.
    milestone_index: MilestoneIndex,
    /// The [`MilestoneId`](MilestoneId) of the milestone.
    milestone_id: MilestoneId,
    /// The timestamp of the milestone.
    milestone_timestamp: MilestoneTimestamp,
    /// The milestone's payload.
    payload: MilestonePayload,
    /// The milestone's sync status.
    is_synced: bool,
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
    /// Creates ledger update indexes.
    pub async fn create_milestone_indexes(&self) -> Result<(), Error> {
        let collection = self.0.collection::<MilestoneDocument>(MilestoneDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "milestone_index": 1 })
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

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "milestone_timestamp": 1 })
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

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "milestone_id": 1 })
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .name("milestone_id_index".to_string())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        Ok(())
    }

    /// Get the [`MilestonePayload`] of a milestone.
    pub async fn get_milestone_payload_by_id(
        &self,
        milestone_id: &MilestoneId,
    ) -> Result<Option<MilestonePayload>, Error> {
        let payload = self
            .0
            .collection::<MilestonePayload>(MilestoneDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "milestone_id": milestone_id } },
                    doc! { "$replaceRoot": { "newRoot": "$payload" } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;

        Ok(payload)
    }

    /// Get [`MilestonePayload`] of a milestone by the [`MilestoneIndex`].
    pub async fn get_milestone_payload(&self, index: MilestoneIndex) -> Result<Option<MilestonePayload>, Error> {
        let payload = self
            .0
            .collection::<MilestonePayload>(MilestoneDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "milestone_index": index } },
                    doc! { "$replaceRoot": { "newRoot": "$payload" } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?;

        Ok(payload)
    }

    /// Get the timestamp of a milestone by the [`MilestoneIndex`].
    pub async fn get_milestone_timestamp(&self, index: MilestoneIndex) -> Result<Option<MilestoneTimestamp>, Error> {
        #[derive(Deserialize)]
        struct TimestampResult {
            milestone_timestamp: MilestoneTimestamp,
        }

        let timestamp = self
            .0
            .collection::<TimestampResult>(MilestoneDocument::COLLECTION)
            .find_one(
                doc! { "milestone_index": index },
                FindOneOptions::builder()
                    .projection(doc! { "milestone_timestamp": 1 })
                    .build(),
            )
            .await?
            .map(|ts| ts.milestone_timestamp);

        Ok(timestamp)
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
            is_synced: Default::default(),
        };

        let mut doc = bson::to_document(&milestone_document)?;
        doc.insert("_id", milestone_document.milestone_id.to_hex());

        self.0
            .collection::<MilestoneDocument>(MilestoneDocument::COLLECTION)
            .update_one(
                doc! { "milestone_index": milestone_index },
                doc! { "$set": doc },
                UpdateOptions::builder().upsert(true).build(),
            )
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

    /// Marks that all [`Block`](crate::types::stardust::block::Block)s of a milestone have been synchronized.
    pub async fn set_sync_status_blocks(&self, index: MilestoneIndex) -> Result<(), Error> {
        self.0
            .collection::<MilestoneDocument>(MilestoneDocument::COLLECTION)
            .update_one(
                doc! { "milestone_index": index },
                doc! { "$set": {
                    "is_synced": true,
                }},
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    /// Retrieves the sync records sorted by their [`MilestoneIndex`].
    async fn get_sorted_milestone_indices_synced(
        &self,
        range: RangeInclusive<MilestoneIndex>,
    ) -> Result<impl Stream<Item = Result<MilestoneIndex, Error>>, Error> {
        #[derive(Deserialize)]
        struct SyncEntry {
            milestone_index: MilestoneIndex,
        }

        self.0
            .collection::<SyncEntry>(MilestoneDocument::COLLECTION)
            .find(
                doc! {
                    "milestone_index": { "$gte": *range.start(), "$lte": *range.end() },
                    "is_synced": { "$eq": true }
                },
                FindOptions::builder()
                    .sort(doc! {"milestone_index": 1u32})
                    .projection(doc! {"milestone_index": 1u32})
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
