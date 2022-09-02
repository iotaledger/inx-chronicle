// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Holds the `MongoDb` type and its config.

use std::borrow::Borrow;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use mongodb::{
    bson::{self, doc, Document},
    error::Error,
    options::{
        AggregateOptions, ClientOptions, CreateIndexOptions, Credential, FindOneOptions, FindOptions,
        InsertManyOptions, InsertOneOptions, ReplaceOptions,
    },
    results::{CreateIndexResult, InsertManyResult, InsertOneResult, UpdateResult},
    Client, Cursor, IndexModel,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

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
        let mut client_options = ClientOptions::parse(&config.connect_url).await?;

        client_options.app_name = Some("Chronicle".to_string());
        client_options.min_pool_size = config.min_pool_size;

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            let credential = Credential::builder()
                .username(username.clone())
                .password(password.clone())
                .build();
            client_options.credential = Some(credential);
        }

        let client = Client::with_options(client_options)?;

        Ok(Self {
            db: client.database(&config.database_name),
            client,
        })
    }

    /// Gets a collection of the provided type.
    pub fn collection<T: MongoDbCollection>(&self) -> T {
        T::instantiate(self, self.db.collection(T::NAME))
    }

    /// Clears all the collections from the database.
    pub async fn clear(&self) -> Result<(), Error> {
        let collections = self.db.list_collection_names(None).await?;

        for c in collections {
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
    /// The minimum amount of connections in the pool.
    pub min_pool_size: Option<u32>,
}

impl Default for MongoDbConfig {
    fn default() -> Self {
        Self {
            connect_url: MongoDb::DEFAULT_CONNECT_URL.to_string(),
            username: None,
            password: None,
            database_name: MongoDb::DEFAULT_NAME.to_string(),
            min_pool_size: None,
        }
    }
}

pub trait MongoDbCollection {
    /// The collection name.
    const NAME: &'static str;
    /// The document schema.
    type Document: Send + Sync;

    fn instantiate(db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self;

    fn collection(&self) -> &mongodb::Collection<Self::Document>;

    fn with_type<T>(&self) -> mongodb::Collection<T> {
        self.collection().clone_with_type()
    }
}

#[async_trait]
pub trait MongoCollectionExt: MongoDbCollection {
    async fn create_index(
        &self,
        index: IndexModel,
        options: impl Into<Option<CreateIndexOptions>> + Send + Sync,
    ) -> Result<CreateIndexResult, Error> {
        self.collection().create_index(index, options).await
    }

    async fn aggregate<T: DeserializeOwned>(
        &self,
        pipeline: impl IntoIterator<Item = Document> + Send + Sync,
        options: impl Into<Option<AggregateOptions>> + Send + Sync,
    ) -> Result<Box<dyn Stream<Item = Result<T, Error>> + Unpin + Send>, Error> {
        Ok(Box::new(
            self.collection()
                .aggregate(pipeline, options)
                .await?
                .map(|doc| Ok(bson::from_document::<T>(doc?)?)),
        ))
    }

    async fn find<T: Send + Sync>(
        &self,
        filter: impl Into<Option<Document>> + Send + Sync,
        options: impl Into<Option<FindOptions>> + Send + Sync,
    ) -> Result<Cursor<T>, Error> {
        self.with_type().find(filter, options).await
    }

    async fn find_one<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        filter: impl Into<Option<Document>> + Send + Sync,
        options: impl Into<Option<FindOneOptions>> + Send + Sync,
    ) -> Result<Option<T>, Error> {
        self.with_type().find_one(filter, options).await
    }

    async fn insert_many<T: Serialize + Send + Sync>(
        &self,
        docs: impl IntoIterator<Item = impl Borrow<T> + Send + Sync> + Send + Sync,
        options: impl Into<Option<InsertManyOptions>> + Send + Sync,
    ) -> Result<InsertManyResult, Error> {
        self.with_type().insert_many(docs, options).await
    }

    async fn insert_one<T: Serialize + Send + Sync>(
        &self,
        doc: impl Borrow<T> + Send + Sync,
        options: impl Into<Option<InsertOneOptions>> + Send + Sync,
    ) -> Result<InsertOneResult, Error> {
        self.with_type().insert_one(doc, options).await
    }
    async fn replace_one<T: Serialize + Send + Sync>(
        &self,
        query: Document,
        replacement: impl Borrow<T> + Send + Sync,
        options: impl Into<Option<ReplaceOptions>> + Send + Sync,
    ) -> Result<UpdateResult, Error> {
        self.with_type().replace_one(query, replacement, options).await
    }
}
impl<T: MongoDbCollection> MongoCollectionExt for T {}

pub struct InsertResult {
    pub ignored: usize,
}

#[async_trait]
pub trait InsertIgnoreDuplicatesExt<T> {
    /// Inserts many records and ignores duplicate key errors.
    async fn insert_many_ignore_duplicates(
        &self,
        docs: impl IntoIterator<Item = impl Borrow<T> + Send + Sync> + Send + Sync,
        options: impl Into<Option<InsertManyOptions>> + Send + Sync,
    ) -> Result<InsertResult, Error>;
}

#[async_trait]
impl<T: MongoCollectionExt + Send + Sync, D: Serialize + Send + Sync> InsertIgnoreDuplicatesExt<D> for T {
    /// Inserts many records and ignores duplicate key errors.
    async fn insert_many_ignore_duplicates(
        &self,
        docs: impl IntoIterator<Item = impl Borrow<D> + Send + Sync> + Send + Sync,
        options: impl Into<Option<InsertManyOptions>> + Send + Sync,
    ) -> Result<InsertResult, Error> {
        use mongodb::error::ErrorKind;
        match self.insert_many(docs, options).await {
            Ok(_) => Ok(InsertResult { ignored: 0 }),
            Err(e) => match &*e.kind {
                ErrorKind::BulkWrite(b) => {
                    if let Some(write_errs) = &b.write_errors {
                        if write_errs.iter().all(|e| e.code == DUPLICATE_KEY_CODE) {
                            return Ok(InsertResult {
                                ignored: write_errs.len(),
                            });
                        }
                    }
                    Err(e)
                }
                _ => Err(e),
            },
        }
    }
}
