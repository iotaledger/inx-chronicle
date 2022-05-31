// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_rest_api_stardust::types::dtos as bee;
use futures::{Stream, TryStreamExt};
use mongodb::{
    bson::{self, doc, DateTime},
    error::Error,
    options::{FindOptions, UpdateOptions},
    results::UpdateResult,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::block::{MilestoneIndex, MilestoneOption};
use crate::db::{
    model::stardust::block::{MilestoneId, MilestonePayload},
    MongoDb,
};

/// A milestone's metadata.
#[derive(Serialize, Deserialize)]
pub struct MilestoneRecord {
    /// The [`MilestoneId`](MilestoneId) of the milestone.
    #[serde(rename = "_id")]
    pub milestone_id: MilestoneId,
    /// The milestone index.
    pub milestone_index: u32,
    /// The timestamp of the milestone.
    pub milestone_timestamp: DateTime,
    /// The milestone's payload.
    pub payload: MilestonePayload,
}

impl MilestoneRecord {
    /// The stardust milestone collection name.
    pub const COLLECTION: &'static str = "stardust_milestones";
}

#[cfg(feature = "inx")]
impl TryFrom<inx::proto::Milestone> for MilestoneRecord {
    type Error = inx::Error;

    fn try_from(value: inx::proto::Milestone) -> Result<Self, Self::Error> {
        let milestone = inx::Milestone::try_from(value)?;
        Ok(Self {
            milestone_index: milestone.milestone_info.milestone_index,
            milestone_timestamp: DateTime::from_millis(milestone.milestone_info.milestone_timestamp as i64 * 1000),
            milestone_id: milestone.milestone_info.milestone_id.into(),
            payload: (&milestone.milestone).into(),
        })
    }
}

impl MongoDb {
    /// Get milestone with index.
    pub async fn get_milestone_record(&self, id: &MilestoneId) -> Result<Option<MilestoneRecord>, Error> {
        self.0
            .collection::<MilestoneRecord>(MilestoneRecord::COLLECTION)
            .find_one(doc! {"_id": bson::to_bson(id)?}, None)
            .await
    }

    /// Get milestone with index.
    pub async fn get_milestone_record_by_index(&self, index: u32) -> Result<Option<MilestoneRecord>, Error> {
        self.0
            .collection::<MilestoneRecord>(MilestoneRecord::COLLECTION)
            .find_one(doc! {"milestone_index": index}, None)
            .await
    }

    /// Upserts a [`MilestoneRecord`] to the database.
    pub async fn upsert_milestone_record(&self, milestone_record: &MilestoneRecord) -> Result<UpdateResult, Error> {
        let doc = bson::to_document(milestone_record)?;
        self.0
            .collection::<MilestoneRecord>(MilestoneRecord::COLLECTION)
            .update_one(
                doc! { "milestone_index": milestone_record.milestone_index },
                doc! { "$set": doc },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
    }

    /// Find the starting milestone.
    pub async fn find_first_milestone(&self, start_timestamp: OffsetDateTime) -> Result<Option<u32>, Error> {
        Ok(self.0.collection::<MilestoneRecord>(MilestoneRecord::COLLECTION).find(
            doc! {"milestone_timestamp": { "$gte": DateTime::from_millis(start_timestamp.unix_timestamp() * 1000) }},
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
    pub async fn find_last_milestone(&self, end_timestamp: OffsetDateTime) -> Result<Option<u32>, Error> {
        Ok(self
            .0
            .collection::<MilestoneRecord>(MilestoneRecord::COLLECTION)
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
            .map(|d| d.milestone_index))
    }

    ///
    pub async fn milestone_records_sorted_with_receipt(
        &self,
    ) -> Result<impl Stream<Item = Result<MilestoneRecord, Error>>, Error> {
        self.0
            .collection::<MilestoneRecord>(MilestoneRecord::COLLECTION)
            .find(
                doc! { "payload.essence.options.receipt.migrated_at": { "$ne": 0} },
                FindOptions::builder().sort(doc! {"milestone_index": 1u32}).build(),
            )
            .await
    }

    ///
    pub async fn milestone_records_sorted_with_receipt_migrated_at(
        &self,
        migrated_at: MilestoneIndex,
    ) -> Result<impl Stream<Item = Result<MilestoneRecord, Error>>, Error> {
        self.0
            .collection::<MilestoneRecord>(MilestoneRecord::COLLECTION)
            .find(
                doc! { "payload.essence.options.receipt.migrated_at": migrated_at },
                FindOptions::builder().sort(doc! {"milestone_index": 1u32}).build(),
            )
            .await
    }

    ///
    pub async fn get_receipts(&self) -> Result<Vec<bee::ReceiptDto>, Error> {
        let mut milestone_records = self.milestone_records_sorted_with_receipt().await?;
        let mut receipt_dtos = vec![];
        while let Some(milestone_record) = milestone_records.try_next().await? {
            receipt_dtos.extend(milestone_record.payload.essence.options.into_iter().cloned().filter_map(|o| {
                let o: &bee_block_stardust::payload::milestone::MilestoneOption = &o.try_into().unwrap();
                let o: bee_block_stardust::payload::milestone::option::dto::MilestoneOptionDto = o.into();
                if let bee_block_stardust::payload::milestone::option::dto::MilestoneOptionDto::Receipt(receipt) = o {
                    Some(bee::ReceiptDto {
                        receipt,
                        milestone_index: milestone_record.milestone_index,
                    })
                } else {
                    None
                }
            }));
        }
        Ok(receipt_dtos)
    }

    ///
    pub async fn get_receipts_migrated_at(&self, migrated_at: MilestoneIndex) -> Result<Vec<bee::ReceiptDto>, Error> {
        let mut milestone_records = self
            .milestone_records_sorted_with_receipt_migrated_at(migrated_at)
            .await?;
        let mut receipt_dtos = vec![];
        while let Some(milestone_record) = milestone_records.try_next().await? {
            receipt_dtos.extend(milestone_record.payload.essence.options.into_iter().cloned().filter_map(|o| {
                let o: &bee_block_stardust::payload::milestone::MilestoneOption = &o.try_into().unwrap();
                let o: bee_block_stardust::payload::milestone::option::dto::MilestoneOptionDto = o.into();
                if let bee_block_stardust::payload::milestone::option::dto::MilestoneOptionDto::Receipt(receipt) = o {
                    Some(bee::ReceiptDto {
                        receipt,
                        milestone_index: milestone_record.milestone_index,
                    })
                } else {
                    None
                }
            }));
        }
        Ok(receipt_dtos)
    }
}
