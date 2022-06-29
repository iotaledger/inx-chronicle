// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Holds the `MongoDb` type and its config.

use mongodb::{
    bson::{doc, Document},
    error::Error,
    options::{ClientOptions, Credential},
    Client,
};
use serde::{Deserialize, Serialize};

/// A handle to the underlying `MongoDB` database.
#[derive(Clone, Debug)]
pub struct MongoDb {
    pub(crate) db: mongodb::Database,
    pub(crate) client: mongodb::Client,
}

impl MongoDb {
    const DEFAULT_NAME: &'static str = "chronicle";
    const DEFAULT_CONNECT_URL: &'static str = "mongodb://localhost:27017";

    /// Constructs a [`MongoDb`] by connecting to a MongoDB instance.
    pub async fn connect(config: &MongoDbConfig) -> Result<MongoDb, Error> {
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

        let db = client.database(&config.database_name);

        Ok(MongoDb { db, client })
    }

    /// Creates a new session.
    pub async fn create_session(&self) -> Result<mongodb::ClientSession, Error> {
        self.client.start_session(None).await
    }

    /// Clears all the collections from the database.
    pub async fn clear(&self) -> Result<(), Error> {
        let collections = self.db.list_collection_names(None).await?;

        for c in collections {
            self.db.collection::<Document>(&c).drop(None).await?;
        }

        Ok(())
    }

    /// Returns the storage size of the database.
    pub async fn size(&self) -> Result<u64, Error> {
        Ok(
            match self
                .db
                .run_command(
                    doc! {
                        "dbStats": 1,
                        "scale": 1,
                        "freeStorage": 0
                    },
                    None,
                )
                .await?
                .get("storageSize")
                .unwrap()
            {
                mongodb::bson::Bson::Int32(i) => *i as u64,
                mongodb::bson::Bson::Int64(i) => *i as u64,
                mongodb::bson::Bson::Double(f) => *f as u64,
                _ => unreachable!(),
            },
        )
    }

    /// Returns the name of the database.
    pub fn name(&self) -> &str {
        self.db.name()
    }
}

/// The [`MongoDb`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MongoDbConfig {
    pub(crate) connect_url: String,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) database_name: String,
}

impl MongoDbConfig {
    /// Creates a new [`MongoDbConfig`].
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

    /// Sets the suffix.
    pub fn with_database_name(mut self, database_name: impl Into<String>) -> Self {
        self.database_name = database_name.into();
        self
    }
}

impl Default for MongoDbConfig {
    fn default() -> Self {
        Self {
            connect_url: MongoDb::DEFAULT_CONNECT_URL.to_string(),
            username: None,
            password: None,
            database_name: MongoDb::DEFAULT_NAME.to_string(),
        }
    }
}
