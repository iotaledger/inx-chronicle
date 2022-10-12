// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Holds the `MongoDb` type and its config.

mod collection;

use std::collections::{HashMap, HashSet};

use mongodb::{
    bson::{doc, Document},
    error::Error,
    options::{ClientOptions, Credential},
    Client,
};
use serde::{Deserialize, Serialize};

pub use self::collection::{InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt};

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
    pub async fn connect(config: &MongoDbConfig) -> Result<Self, Error> {
        let mut client_options = ClientOptions::parse(&config.conn_str).await?;

        client_options.app_name = Some("Chronicle".to_string());
        client_options.min_pool_size = config.min_pool_size;

        if client_options.credential.is_none() {
            if let (Some(username), Some(password)) = (&config.username, &config.password) {
                let credential = Credential::builder()
                    .username(username.clone())
                    .password(password.clone())
                    .build();
                client_options.credential = Some(credential);
            }
        }

        let client = Client::with_options(client_options)?;

        Ok(Self {
            db: client.database(&config.database_name),
            client,
        })
    }

    /// Creates a collection if it does not exist.
    pub async fn create_indexes<T: MongoDbCollection + Send + Sync>(&self) -> Result<(), Error> {
        let collection = self.collection::<T>();
        collection.create_collection(&self).await?;
        collection.create_indexes().await?;
        Ok(())
    }

    /// Gets a collection of the provided type.
    pub fn collection<T: MongoDbCollection>(&self) -> T {
        T::instantiate(self, self.db.collection(T::NAME))
    }

    /// Gets all index names by their collection.
    pub async fn get_index_names(&self) -> Result<HashMap<String, HashSet<String>>, Error> {
        let mut res = HashMap::new();
        for collection in self.db.list_collection_names(None).await? {
            let indexes = self.db.collection::<Document>(&collection).list_index_names().await?;
            if !indexes.is_empty() {
                res.insert(collection, indexes.into_iter().collect());
            }
        }
        Ok(res)
    }

    /// Clears all the collections from the database.
    pub async fn clear(&self) -> Result<(), Error> {
        let collections = self.db.list_collection_names(None).await?;

        for c in collections.into_iter().filter(|c| c != "system.views") {
            self.db.collection::<Document>(&c).drop(None).await?;
        }

        Ok(())
    }

    /// Drops the database.
    pub async fn drop(self) -> Result<(), Error> {
        self.db.drop(None).await
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

/// The [`MongoDb`] config.
#[must_use]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MongoDbConfig {
    /// The bind address of the database.
    pub conn_str: String,
    /// The MongoDB username.
    pub username: Option<String>,
    /// The MongoDB password.
    pub password: Option<String>,
    /// The name of the database to connect to.
    pub database_name: String,
    /// The minimum amount of connections in the pool.
    pub min_pool_size: Option<u32>,
}

impl Default for MongoDbConfig {
    fn default() -> Self {
        Self {
            conn_str: MongoDb::DEFAULT_CONNECT_URL.to_string(),
            username: None,
            password: None,
            database_name: MongoDb::DEFAULT_NAME.to_string(),
            min_pool_size: None,
        }
    }
}
