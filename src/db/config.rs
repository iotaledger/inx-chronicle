// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! MongoDB configuration.

use mongodb::{
    bson::doc,
    options::{ClientOptions, Credential},
    Client,
};
use serde::{Deserialize, Serialize};

use super::{MongoDatabase, MongoDbError, DB_NAME};

const LOCATION_DEFAULT: &str = "mongodb://localhost:27017";

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

impl Default for MongoConfig {
    fn default() -> Self {
        Self::new(LOCATION_DEFAULT)
    }
}
