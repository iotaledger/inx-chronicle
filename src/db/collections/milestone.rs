// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{FindOptions, UpdateOptions},
    results::UpdateResult,
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
pub struct MilestoneDocument {
    /// The [`MilestoneId`](MilestoneId) of the milestone.
    #[serde(rename = "_id")]
    pub milestone_id: MilestoneId,
    /// The milestone index.
    pub milestone_index: MilestoneIndex,
    /// The timestamp of the milestone.
    pub milestone_timestamp: MilestoneTimestamp,
    /// The milestone's payload.
    pub payload: MilestonePayload,
}

impl MilestoneDocument {
    /// The stardust milestone collection name.
    pub const COLLECTION: &'static str = "stardust_milestones";
}

#[cfg(feature = "inx")]
impl TryFrom<inx::proto::Milestone> for MilestoneDocument {
    type Error = inx::Error;

    fn try_from(value: inx::proto::Milestone) -> Result<Self, Self::Error> {
        let milestone = inx::Milestone::try_from(value)?;
        Ok(Self {
            milestone_index: milestone.milestone_info.milestone_index.into(),
            milestone_timestamp: milestone.milestone_info.milestone_timestamp.into(),
            milestone_id: milestone.milestone_info.milestone_id.into(),
            payload: (&milestone.milestone).into(),
        })
    }
}

impl MongoDb {
    /// Get milestone with index.
    pub async fn get_milestone_record(&self, id: &MilestoneId) -> Result<Option<MilestoneDocument>, Error> {
        self.0
            .collection::<MilestoneDocument>(MilestoneDocument::COLLECTION)
            .find_one(doc! {"_id": bson::to_bson(id)?}, None)
            .await
    }

    /// Get milestone with index.
    pub async fn get_milestone_record_by_index(
        &self,
        index: MilestoneIndex,
    ) -> Result<Option<MilestoneDocument>, Error> {
        self.0
            .collection::<MilestoneDocument>(MilestoneDocument::COLLECTION)
            .find_one(doc! {"milestone_index": index}, None)
            .await
    }

    /// Upserts a [`MilestoneRecord`] to the database.
    pub async fn upsert_milestone_record(&self, milestone_record: &MilestoneDocument) -> Result<UpdateResult, Error> {
        let doc = bson::to_document(milestone_record)?;
        self.0
            .collection::<MilestoneDocument>(MilestoneDocument::COLLECTION)
            .update_one(
                doc! { "milestone_index": milestone_record.milestone_index },
                doc! { "$set": doc },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
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
}
