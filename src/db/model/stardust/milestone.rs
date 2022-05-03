// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::payload::{milestone::MilestoneId, MilestonePayload};
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, DateTime, Document},
    error::Error,
    options::{FindOptions, UpdateOptions},
    results::UpdateResult,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::db::{
    bson::{self, DocExt},
    MongoDb,
};

/// A milestone's metadata.
#[derive(Serialize, Deserialize)]
pub struct MilestoneRecord {
    /// The milestone index.
    pub milestone_index: u32,
    /// The timestamp of the milestone.
    pub milestone_timestamp: DateTime,
    /// The [`MilestoneId`] of the milestone.
    pub milestone_id: MilestoneId,
    /// The milestone's payload.
    pub payload: MilestonePayload,
}

impl MilestoneRecord {
    /// The stardust milestone collection name.
    pub const COLLECTION: &'static str = "stardust_milestones";
}

impl TryFrom<inx::proto::Milestone> for MilestoneRecord {
    type Error = inx::Error;

    fn try_from(value: inx::proto::Milestone) -> Result<Self, Self::Error> {
        let milestone = inx::Milestone::try_from(value)?;
        Ok(Self {
            milestone_index: milestone.milestone_info.milestone_index,
            milestone_timestamp: DateTime::from_millis(milestone.milestone_info.milestone_timestamp as i64 * 1000),
            milestone_id: milestone.milestone_info.milestone_id,
            payload: milestone.milestone,
        })
    }
}

impl MongoDb {
    /// Get milestone with index.
    pub async fn get_milestone_record_by_index(&self, index: u32) -> Result<Option<Document>, Error> {
        let res = self
            .0
            .collection::<Document>(MilestoneRecord::COLLECTION)
            .find_one(doc! {"milestone_index": index}, None)
            .await;

        Ok(res.unwrap()) // Fix the `DocErr` type
    }

    /// Upserts a [`MilestoneRecord`] to the database.
    pub async fn upsert_milestone_record(&self, milestone_record: &MilestoneRecord) -> Result<UpdateResult, Error> {
        let doc = bson::to_document(milestone_record)?;
        self.0
            .collection::<Document>(MilestoneRecord::COLLECTION)
            .update_one(
                doc! { "milestone_index": milestone_record.milestone_index },
                doc! { "$set": doc },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
    }

    /// Find the starting milestone.
    pub async fn find_first_milestone(&self, start_timestamp: OffsetDateTime) -> Result<Option<u32>, Error> {
        let res = self.0.collection::<Document>(MilestoneRecord::COLLECTION).find(
            doc! {"milestone_timestamp": { "$gte": DateTime::from_millis(start_timestamp.unix_timestamp() * 1000) }},
            FindOptions::builder()
                .sort(doc! {"milestone_index": 1u32})
                .limit(1)
                .build(),
        )
        .await?
        .try_next()
        .await?
        .map(|d| d.get_as_u32("milestone_index"))
        .transpose();

        Ok(res.unwrap()) // Fix the `DocErr` type
    }

    /// Find the end milestone.
    pub async fn find_last_milestone(&self, end_timestamp: OffsetDateTime) -> Result<Option<u32>, Error> {
        let res = self
            .0
            .collection::<Document>(MilestoneRecord::COLLECTION)
            .find(
                doc! {"milestone_timestamp": { "$lte": DateTime::from_millis(end_timestamp.unix_timestamp() * 1000) }},
                FindOptions::builder()
                    .sort(doc! {"milestone_index": -1})
                    .limit(1)
                    .build(),
            )
            .await?
            .try_next()
            .await?
            .map(|d| d.get_as_u32("milestone_index"))
            .transpose();

        Ok(res.unwrap()) // Fix the `DocErr` type
    }
}
