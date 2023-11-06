// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::slot::{SlotCommitmentId, SlotIndex};
use mongodb::{
    bson::doc,
    options::{FindOneOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        mongodb::{DbError, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::SerializeToBson,
};

/// The corresponding MongoDb document representation to store committed slots.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommittedSlotDocument {
    #[serde(rename = "_id")]
    pub slot_index: SlotIndex,
    pub commitment_id: SlotCommitmentId,
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

    /// Inserts or updates a committed slot.
    pub async fn upsert_committed_slot(
        &self,
        slot_index: SlotIndex,
        commitment_id: SlotCommitmentId,
    ) -> Result<(), DbError> {
        self.update_one(
            doc! { "_id": slot_index.0 },
            doc! { "$set": {
                    "commitment_id": commitment_id.to_bson()
                }
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }
}
