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

/// Name of the MongoDB database.
pub const DB_NAME: &str = "chronicle-test";

/// A builder to establish a connection to the database.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MongoConfig {
    location: String,
    username: Option<String>,
    password: Option<String>,
}

impl MongoConfig {
    /// Creates a new [`MongoConfig`]. The `location` is the address of the MongoDB instance.
    pub fn new<S: Into<String>>(location: S) -> Self {
        Self {
            location: location.into(),
            username: None,
            password: None,
        }
    }

    /// Sets the username.
    pub fn with_username<S: Into<String>>(mut self, username: S) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Sets the password.
    pub fn with_password<S: Into<String>>(mut self, password: S) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Constructs a [`MongoDatabase`] by consuming the [`MongoConfig`].
    pub async fn build(&self) -> Result<MongoDatabase, MongoDbError> {
        let mut client_options = ClientOptions::parse(&self.location).await?;

        client_options.app_name = Some("Chronicle".to_string());

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            let credential = Credential::builder()
                .username(username.clone())
                .password(password.clone())
                .build();
            client_options.credential = Some(credential);
        }

        let client = Client::with_options(client_options)?;
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
