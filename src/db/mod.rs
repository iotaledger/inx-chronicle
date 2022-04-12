// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// The names of the collections in the MongoDB database.
mod error;
/// The models of the data stored in the database.
pub mod model;

use mongodb::{
    options::{ClientOptions, Credential},
    Client,
};
use serde::{Deserialize, Serialize};

pub use self::{error::MongoDbError, model::Model};

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
    pub async fn into_db(self) -> Result<MongoDatabase, MongoDbError> {
        let mut client_options = ClientOptions::parse(self.location).await?;

        client_options.app_name = Some("Chronicle".to_string());

        if let (Some(username), Some(password)) = (self.username, self.password) {
            let credential = Credential::builder().username(username).password(password).build();
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
    pub async fn insert_one<M: Model>(&self, model: M) -> Result<(), MongoDbError> {
        self.db.collection::<M>(M::COLLECTION).insert_one(&model, None).await?;
        Ok(())
    }
}
