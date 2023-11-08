// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use iota_sdk::types::block::slot::{SlotCommitment, SlotCommitmentId, SlotIndex};
use mongodb::{
    bson::doc,
    options::{FindOneOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};

use super::SortOrder;
use crate::{
    db::{
        mongodb::{DbError, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::{raw::Raw, SerializeToBson},
};

/// The corresponding MongoDb document representation to store committed slots.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommittedSlotDocument {
    #[serde(rename = "_id")]
    pub slot_index: SlotIndex,
    pub commitment_id: SlotCommitmentId,
    pub commitment: Raw<SlotCommitment>,
}

/// A collection to store committed slots.
pub struct CommittedSlotCollection {
    collection: mongodb::Collection<CommittedSlotDocument>,
}

impl MongoDbCollection for CommittedSlotCollection {
    const NAME: &'static str = "iota_committed_slots";
    type Document = CommittedSlotDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

impl CommittedSlotCollection {
    /// Gets the latest committed slot.
    pub async fn get_latest_committed_slot(&self) -> Result<Option<CommittedSlotDocument>, DbError> {
        Ok(self
            .find_one(doc! {}, FindOneOptions::builder().sort(doc! { "_id": -1 }).build())
            .await?)
    }

    /// Gets the [`SlotCommitmentId`] for the given slot index.
    pub async fn get_id_for_slot_index(&self, slot_index: SlotIndex) -> Result<Option<SlotCommitmentId>, DbError> {
        Ok(self
            .find_one::<CommittedSlotDocument>(doc! { "_id": slot_index.0 }, None)
            .await?
            .map(|doc| doc.commitment_id))
    }

    /// Gets the committed slot for the given slot index.
    pub async fn get_commitment(&self, index: SlotIndex) -> Result<Option<CommittedSlotDocument>, DbError> {
        Ok(self
            .find_one::<CommittedSlotDocument>(doc! { "_id": index.0 }, None)
            .await?)
    }

    /// Gets the paged committed slots for the given slot index range.
    pub async fn get_commitments(
        &self,
        start_index: Option<SlotIndex>,
        end_index: Option<SlotIndex>,
        sort: SortOrder,
        page_size: usize,
        cursor: Option<SlotIndex>,
    ) -> Result<impl Stream<Item = Result<CommittedSlotDocument, DbError>>, DbError> {
        let (sort, cmp) = match sort {
            SortOrder::Newest => (doc! {"_id": -1 }, "$lte"),
            SortOrder::Oldest => (doc! {"_id": 1 }, "$gte"),
        };

        let mut queries = Vec::new();
        if let Some(start_index) = start_index {
            queries.push(doc! { "_id": { "$gte": start_index.0 } });
        }
        if let Some(end_index) = end_index {
            queries.push(doc! { "_id": { "$lte": end_index.0 } });
        }
        if let Some(index) = cursor {
            queries.push(doc! { "_id": { cmp: index.0 } });
        }

        Ok(self
            .aggregate(
                [
                    doc! { "$match": { "$and": queries } },
                    doc! { "$sort": sort },
                    doc! { "$limit": page_size as i64 },
                ],
                None,
            )
            .await?
            .map_err(Into::into))
    }

    /// Inserts or updates a committed slot.
    pub async fn upsert_committed_slot(
        &self,
        slot_index: SlotIndex,
        commitment_id: SlotCommitmentId,
        commitment: Raw<SlotCommitment>,
    ) -> Result<(), DbError> {
        self.update_one(
            doc! { "_id": slot_index.0 },
            doc! { "$set": {
                    "commitment_id": commitment_id.to_bson(),
                    "commitment": commitment.to_bson()
                }
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }
}
