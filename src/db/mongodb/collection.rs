// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use mongodb::{
    bson::{self, doc, Document},
    error::Error,
    options::{
        AggregateOptions, CreateIndexOptions, FindOneOptions, FindOptions, InsertManyOptions, InsertOneOptions,
        ReplaceOptions,
    },
    results::{CreateIndexResult, InsertManyResult, InsertOneResult, UpdateResult},
    Cursor, IndexModel,
};
use serde::{de::DeserializeOwned, Serialize};

use super::{MongoDb, DUPLICATE_KEY_CODE};

/// A MongoDB collection.
pub trait MongoDbCollection {
    /// The collection name.
    const NAME: &'static str;
    /// The document schema.
    type Document: Send + Sync;

    /// Creates an instance of this collection type.
    fn instantiate(db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self;

    /// Gets the underlying MongoDB collection. This must return a collection of the type
    /// specified by this trait, which will be coerced if necessary.
    fn collection(&self) -> &mongodb::Collection<Self::Document>;

    /// Coerce the underlying collection to the needed type.
    fn with_type<T>(&self) -> mongodb::Collection<T> {
        self.collection().clone_with_type()
    }
}

/// An extension trait which wraps the basic functionality of a mongodb
/// [`Collection`](mongodb::Collection) that coerces the document type
/// into the provided generic.
#[async_trait]
pub trait MongoDbCollectionExt: MongoDbCollection {
    async fn create_index(
        &self,
        index: IndexModel,
        options: impl Into<Option<CreateIndexOptions>> + Send + Sync,
    ) -> Result<CreateIndexResult, Error> {
        self.collection().create_index(index, options).await
    }

    /// Calls [`mongodb::Collection::aggregate()`] and coerces the document type.
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

    /// Calls [`mongodb::Collection::find()`] and coerces the document type.
    async fn find<T: Send + Sync>(
        &self,
        filter: impl Into<Option<Document>> + Send + Sync,
        options: impl Into<Option<FindOptions>> + Send + Sync,
    ) -> Result<Cursor<T>, Error> {
        self.with_type().find(filter, options).await
    }

    /// Calls [`mongodb::Collection::find_one()`] and coerces the document type.
    async fn find_one<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        filter: impl Into<Option<Document>> + Send + Sync,
        options: impl Into<Option<FindOneOptions>> + Send + Sync,
    ) -> Result<Option<T>, Error> {
        self.with_type().find_one(filter, options).await
    }

    /// Calls [`mongodb::Collection::insert_many()`] and coerces the document type.
    async fn insert_many<T: Serialize + Send + Sync>(
        &self,
        docs: impl IntoIterator<Item = impl Borrow<T> + Send + Sync> + Send + Sync,
        options: impl Into<Option<InsertManyOptions>> + Send + Sync,
    ) -> Result<InsertManyResult, Error> {
        self.with_type().insert_many(docs, options).await
    }

    /// Calls [`mongodb::Collection::insert_one()`] and coerces the document type.
    async fn insert_one<T: Serialize + Send + Sync>(
        &self,
        doc: impl Borrow<T> + Send + Sync,
        options: impl Into<Option<InsertOneOptions>> + Send + Sync,
    ) -> Result<InsertOneResult, Error> {
        self.with_type().insert_one(doc, options).await
    }

    /// Calls [`mongodb::Collection::replace_one()`] and coerces the document type.
    async fn replace_one<T: Serialize + Send + Sync>(
        &self,
        query: Document,
        replacement: impl Borrow<T> + Send + Sync,
        options: impl Into<Option<ReplaceOptions>> + Send + Sync,
    ) -> Result<UpdateResult, Error> {
        self.with_type().replace_one(query, replacement, options).await
    }
}
impl<T: MongoDbCollection> MongoDbCollectionExt for T {}

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
impl<T: MongoDbCollectionExt + Send + Sync, D: Serialize + Send + Sync> InsertIgnoreDuplicatesExt<D> for T {
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
