// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Holds the `MongoDb` type and its config.

use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, Credential, UpdateOptions},
    Client, Collection,
};
use serde::{Deserialize, Serialize};

use super::{model::Model, MongoDbError};

/// Name of the MongoDB database.
pub const DB_NAME: &str = "chronicle-test";
const CONNECT_URL_DEFAULT: &str = "mongodb://localhost:27017";

/// A handle to the underlying `MongoDB` database.
#[derive(Clone, Debug)]
pub struct MongoDb(mongodb::Database);

impl MongoDb {
    /// Constructs a [`MongoDb`] by consuming the builder.
    pub async fn connect(config: &MongoDbConfig) -> Result<MongoDb, MongoDbError> {
        let mut client_options = ClientOptions::parse(&config.connect_url).await?;

        client_options.app_name = Some("Chronicle".to_string());

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            let credential = Credential::builder()
                .username(username.clone())
                .password(password.clone())
                .build();
            client_options.credential = Some(credential);
        }

        let client = Client::with_options(client_options)?;
        let db = client.database(DB_NAME);

        Ok(MongoDb(db))
    }

    /// Inserts a record of a [`Model`] into the database.
    pub async fn upsert_one<M: Model>(&self, model: M) -> Result<(), MongoDbError> {
        let doc = crate::bson::to_document(&model)?;
        self.0
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
        self.0.collection(M::COLLECTION)
    }

    /// Gets a model type's collection.
    pub fn doc_collection<M: Model>(&self) -> Collection<Document> {
        self.0.collection(M::COLLECTION)
    }

}

/// The [`MongoDb`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MongoDbConfig {
    pub(crate) connect_url: String,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
}

impl MongoDbConfig {
    /// Creates a new [`MongoConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the connect URL.
    pub fn with_connect_url(mut self, connect_url: impl Into<String>) -> Self {
        self.connect_url = connect_url.into();
        self
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

}

impl Default for MongoDbConfig {
    fn default() -> Self {
        Self {
            connect_url: CONNECT_URL_DEFAULT.to_string(),
            username: None,
            password: None,
        }
    }
}
