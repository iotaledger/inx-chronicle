// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// The names of the collections in the MongoDB database.
mod collections;
mod error;

pub mod config;
pub use error::MongoDbError;
use inx::proto::{Message, Milestone};
use mongodb::{
    bson,
    bson::{doc, Document},
};

/// Name of the MongoDB database.
pub const DB_NAME: &str = "chronicle-test";

/// A handle to the underlying MongoDB database.
#[derive(Clone, Debug)]
pub struct MongoDatabase {
    db: mongodb::Database,
}

impl MongoDatabase {
    /// Inserts the raw bytes of a [`Message`].
    pub async fn insert_message_raw(&self, message: Message) -> Result<(), MongoDbError> {
        let message_id = &message.message_id.unwrap().id;
        let message = &message.message.unwrap().data;

        self.db
            .collection::<Document>(collections::stardust::raw::MESSAGES)
            .insert_one(
                doc! {
                    "message_id": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message_id.clone()},
                    "raw_message": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message.clone()},
                },
                None,
            )
            .await?;

        Ok(())
    }

    /// Inserts a [`Milestone`].
    pub async fn insert_milestone(&self, milestone: Milestone) -> Result<(), MongoDbError> {
        let milestone_index = milestone.milestone_index;
        let milestone_timestamp = milestone.milestone_timestamp;
        let message_id = &milestone.message_id.unwrap().id;

        self.db
            .collection::<Document>(collections::stardust::MILESTONES)
            .insert_one(
                doc! {
                    "milestone_index": bson::to_bson(&milestone_index).unwrap(),
                    "milestone_timestamp": bson::to_bson(&milestone_timestamp).unwrap(),
                    "message_id": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes: message_id.clone()},
                },
                None,
            )
            .await?;

        Ok(())
    }
}
