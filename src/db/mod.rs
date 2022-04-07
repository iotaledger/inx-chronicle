// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// The names of the collections in the MongoDB database.
mod collections;
mod error;
pub use error::MongoDbError;
use inx::proto::{Message, Milestone};
use mongodb::{
    bson,
    bson::{doc, Document},
    options::{ClientOptions, Credential},
    Client,
};
use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Name of the MongoDB database.
pub const DB_NAME: &str = "chronicle-test";

/// A builder to establish a connection to the database.
#[must_use]
#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MongoConfig {
    location: String,
    username: Option<String>,
    password: Option<String>,
}

impl MongoConfig {
    /// Creates a new [`MongoConfig`]. The `location` is the address of the MongoDB instance.
    pub fn new(location: String) -> Self {
        Self {
            location,
            username: None,
            password: None,
        }
    }

    /// Sets the username.
    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    /// Sets the password.
    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    /// Constructs a [`MongoDatabase`] by consuming the [`MongoConfig`].
    pub async fn build(self) -> Result<MongoDatabase, Error> {
        let mut client_options = ClientOptions::parse(self.location)
            .await
            .map_err(MongoDbError::InvalidClientOptions)?;

        client_options.app_name = Some("Chronicle".to_string());

        if let (Some(username), Some(password)) = (self.username, self.password) {
            let credential = Credential::builder().username(username).password(password).build();
            client_options.credential = Some(credential);
        }

        let client = Client::with_options(client_options).map_err(MongoDbError::InvalidClientOptions)?;
        let db = client.database(DB_NAME);
        Ok(MongoDatabase { db })
    }
}

/// A handle to the underlying MongoDB database.
#[derive(Clone, Debug)]
pub struct MongoDatabase {
    db: mongodb::Database,
}

impl MongoDatabase {
    /// Inserts the raw bytes of a [`Message`].
    pub async fn insert_message_raw(&self, message: Message) -> Result<(), Error> {
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
            .await
            .map_err(MongoDbError::InsertError)?;

        Ok(())
    }

    /// Inserts a [`Milestone`].
    pub async fn insert_milestone(&self, milestone: Milestone) -> Result<(), Error> {
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
            .await
            .map_err(MongoDbError::InsertError)?;

        Ok(())
    }
}
