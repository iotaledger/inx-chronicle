// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Holds the `MongoDb` type and its config.

use std::borrow::Borrow;

use mongodb::{
    bson::{doc, Document},
    error::Error,
    options::{ClientOptions, Credential, InsertManyOptions, TransactionOptions},
    Client, ClientSession,
};
use serde::{Deserialize, Serialize};
use tracing::info;

const DUPLICATE_KEY_CODE: i32 = 11000;

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

    /// Gets a collection of the provided type with the given name.
    pub fn collection<T>(&self, name: impl AsRef<str>) -> MongoDbCollection<T> {
        MongoDbCollection(self.db.collection(name.as_ref()))
    }

    /// Starts a transaction.
    pub async fn start_transaction(
        &self,
        options: impl Into<Option<TransactionOptions>>,
    ) -> Result<ClientSession, Error> {
        let mut session = self.client.start_session(None).await?;
        session.start_transaction(options).await?;
        Ok(session)
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

    /// Returns the names of all available databases.
    pub async fn get_databases(&self) -> Result<Vec<String>, Error> {
        self.client.list_database_names(None, None).await
    }

    /// Returns the name of the database.
    pub fn name(&self) -> &str {
        self.db.name()
    }
}

pub struct MongoDbCollection<T>(mongodb::Collection<T>);

impl<T: Serialize> MongoDbCollection<T> {
    /// Inserts many records and ignores duplicate key errors.
    pub async fn insert_many_ignore_duplicates(
        &self,
        docs: impl IntoIterator<Item = impl Borrow<T>>,
        options: impl Into<Option<InsertManyOptions>>,
    ) -> Result<usize, Error> {
        use mongodb::error::ErrorKind;
        match self.insert_many(docs, options).await {
            Ok(r) => Ok(r.inserted_ids.len()),
            Err(e) => match &*e.kind {
                ErrorKind::BulkWrite(b) => {
                    if let Some(write_errs) = &b.write_errors {
                        if write_errs.len() == 1 {
                            if write_errs[0].code == DUPLICATE_KEY_CODE {
                                info!("{:?}", e);
                                return Ok(0);
                            }
                        }
                    }
                    Err(e)
                }
                _ => Err(e),
            },
        }
    }
}

impl<T> std::ops::Deref for MongoDbCollection<T> {
    type Target = mongodb::Collection<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The [`MongoDb`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MongoDbConfig {
    /// The bind address of the database.
    pub connect_url: String,
    /// The MongoDB username.
    pub username: Option<String>,
    /// The MongoDB password.
    pub password: Option<String>,
    /// The name of the database to connect to.
    pub database_name: String,
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
