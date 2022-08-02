// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeInclusive;

use futures::{Stream, StreamExt, TryStreamExt};
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{FindOneOptions, FindOptions, IndexOptions},
    ClientSession, IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        ledger::MilestoneIndexTimestamp,
        stardust::{
            block::{MilestoneId, MilestoneOption, MilestonePayload},
            milestone::MilestoneTimestamp,
        },
        tangle::MilestoneIndex,
    },
};

/// A milestone's metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct MilestoneDocument {
    /// The milestone index and timestamp.
    at: MilestoneIndexTimestamp,
    /// The [`MilestoneId`](MilestoneId) of the milestone.
    milestone_id: MilestoneId,
    /// The milestone's payload.
    payload: MilestonePayload,
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
        let collection = self.db.collection::<MilestoneDocument>(MilestoneDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "at.milestone_index": 1 })
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
                    .keys(doc! { "at.milestone_timestamp": 1 })
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

    /// Gets the [`MilestonePayload`] of a milestone.
    pub async fn get_milestone_payload_by_id(
        &self,
        milestone_id: &MilestoneId,
    ) -> Result<Option<MilestonePayload>, Error> {
        Ok(self
            .db
            .collection::<MilestonePayload>(MilestoneDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "milestone_id": milestone_id } },
                    doc! { "$replaceWith": "$payload" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?)
    }

    /// Gets [`MilestonePayload`] of a milestone by the [`MilestoneIndex`].
    pub async fn get_milestone_payload(&self, index: MilestoneIndex) -> Result<Option<MilestonePayload>, Error> {
        Ok(self
            .db
            .collection::<MilestonePayload>(MilestoneDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$match": { "at.milestone_index": index } },
                    doc! { "$replaceWith": "$payload" },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(bson::from_document)
            .transpose()?)
    }

    /// Gets the timestamp of a milestone by the [`MilestoneIndex`].
    pub async fn get_milestone_timestamp(&self, index: MilestoneIndex) -> Result<Option<MilestoneTimestamp>, Error> {
        Ok(self
            .db
            .collection::<MilestoneIndexTimestamp>(MilestoneDocument::COLLECTION)
            .find_one(
                doc! { "at.milestone_index": index },
                FindOneOptions::builder()
                    .projection(doc! {
                        "milestone_index": "$at.milestone_index",
                        "milestone_timestamp": "$at.milestone_timestamp",
                    })
                    .build(),
            )
            .await?
            .map(|ts| ts.milestone_timestamp))
    }

    /// Inserts the information of a milestone into the database.
    pub async fn insert_milestone(
        &self,
        session: &mut ClientSession,
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

        let mut doc = bson::to_document(&milestone_document)?;
        doc.insert("_id", milestone_document.milestone_id.to_hex());

        self.db
            .collection::<bson::Document>(MilestoneDocument::COLLECTION)
            .insert_one_with_session(doc, None, session)
            .await?;

        Ok(())
    }

    /// Find the starting milestone.
    pub async fn find_first_milestone(
        &self,
        start_timestamp: MilestoneTimestamp,
    ) -> Result<Option<MilestoneIndexTimestamp>, Error> {
        self.db
            .collection::<MilestoneIndexTimestamp>(MilestoneDocument::COLLECTION)
            .find(
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
        self.db
            .collection::<MilestoneIndexTimestamp>(MilestoneDocument::COLLECTION)
            .find(
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

    /// Find the latest milestone inserted.
    pub async fn get_latest_milestone(&self) -> Result<Option<MilestoneIndexTimestamp>, Error> {
        self.db
            .collection::<MilestoneIndexTimestamp>(MilestoneDocument::COLLECTION)
            .find(
                doc! {},
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

    /// Gets the current ledger index.
    pub async fn get_ledger_index(&self) -> Result<Option<MilestoneIndex>, Error> {
        Ok(self.get_latest_milestone().await?.map(|ts| ts.milestone_index))
    }

    /// Retrieves the sync records sorted by their [`MilestoneIndex`].
    async fn get_sorted_milestone_indices_synced(
        &self,
        range: RangeInclusive<MilestoneIndex>,
    ) -> Result<impl Stream<Item = Result<MilestoneIndex, Error>>, Error> {
        Ok(self
            .db
            .collection::<MilestoneIndexTimestamp>(MilestoneDocument::COLLECTION)
            .find(
                doc! {
                    "at.milestone_index": { "$gte": *range.start(), "$lte": *range.end() },
                },
                FindOptions::builder()
                    .sort(doc! { "at.milestone_index": 1 })
                    .projection(doc! {
                        "milestone_index": "$at.milestone_index",
                        "milestone_timestamp": "$at.milestone_timestamp",
                    })
                    .build(),
            )
            .await?
            .map_ok(|ts| ts.milestone_index))
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

    /// Retrieves gaps in the milestones collection.
    pub async fn get_gaps(&self) -> Result<Vec<RangeInclusive<MilestoneIndex>>, Error> {
        let mut synced_ms = self
            .db
            .collection::<MilestoneIndexTimestamp>(MilestoneDocument::COLLECTION)
            .find(
                doc! {},
                FindOptions::builder()
                    .sort(doc! { "at.milestone_index": 1 })
                    .projection(doc! {
                        "milestone_index": "$at.milestone_index",
                        "milestone_timestamp": "$at.milestone_timestamp",
                    })
                    .build(),
            )
            .await?
            .map_ok(|e| e.milestone_index);

        let mut gaps = Vec::new();
        let mut last_record: Option<MilestoneIndex> = None;

        while let Some(milestone_index) = synced_ms.try_next().await? {
            // Missing records go into gaps
            if let Some(&last) = last_record.as_ref() {
                if last + 1 < milestone_index {
                    gaps.push(last + 1..=milestone_index - 1);
                }
            }
            last_record.replace(milestone_index);
        }
        Ok(gaps)
    }

    /// Streams all available receipt milestone options together with their corresponding `MilestoneIndex`.
    pub async fn stream_all_receipts(
        &self,
    ) -> Result<impl Stream<Item = Result<(MilestoneOption, MilestoneIndex), Error>>, Error> {
        #[derive(Deserialize)]
        struct ReceiptAtIndex {
            receipt: MilestoneOption,
            index: MilestoneIndex,
        }

        Ok(self
            .db
            .collection::<ReceiptAtIndex>(MilestoneDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$unwind": "payload.essence.options"},
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
            .map(|doc| {
                let ReceiptAtIndex { receipt, index } = bson::from_document::<ReceiptAtIndex>(doc?)?;
                Ok((receipt, index))
            }))
    }

    /// Streams all available receipt milestone options together with their corresponding `MilestoneIndex` that were
    /// migrated at the given index.
    pub async fn stream_receipts_migrated_at(
        &self,
        migrated_at: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<(MilestoneOption, MilestoneIndex), Error>>, Error> {
        #[derive(Deserialize)]
        struct ReceiptAtIndex {
            receipt: MilestoneOption,
            index: MilestoneIndex,
        }

        Ok(self
            .db
            .collection::<ReceiptAtIndex>(MilestoneDocument::COLLECTION)
            .aggregate(
                vec![
                    doc! { "$unwind": "payload.essence.options"},
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
            .map(|doc| {
                let ReceiptAtIndex { receipt, index } = bson::from_document::<ReceiptAtIndex>(doc?)?;
                Ok((receipt, index))
            }))
    }
}
