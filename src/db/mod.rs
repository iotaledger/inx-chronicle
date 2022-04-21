// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod error;
/// Module containing database record models.
pub mod model;

pub use error::MongoDbError;
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, Credential, UpdateOptions},
    Client, Collection,
};
use serde::{Deserialize, Serialize};

use self::model::Model;

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
    pub fn new(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            username: None,
            password: None,
        }
    }

    /// Sets the username.
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Sets the password.
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
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
    /// Inserts a record of a [`Model`] into the database.
    pub async fn upsert_one<M: Model>(&self, model: M) -> Result<(), MongoDbError> {
        let doc = crate::bson::to_document(&model)?;
        self.db
            .collection::<Document>(M::COLLECTION)
            .update_one(
                model.key(),
                doc! { "$set": doc },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;
        Ok(())
    }

    /// Gets a model type's collection.
    pub fn collection<M: Model>(&self) -> Collection<M> {
        self.db.collection(M::COLLECTION)
    }

    /// Gets a model type's collection.
    pub fn doc_collection<M: Model>(&self) -> Collection<Document> {
        self.db.collection(M::COLLECTION)
    }
}
