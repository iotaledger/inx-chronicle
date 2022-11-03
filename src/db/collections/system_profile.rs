// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::Stream;
use mongodb::{
    bson::{doc, DateTime},
    error::Error,
};
use serde::Deserialize;

use crate::db::{MongoDb, MongoDbCollection, MongoDbCollectionExt};

/// MongoDb `system.profile` document.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct SystemProfileDocument {
    #[serde(rename = "op")]
    pub operation: String,
    #[serde(rename = "ns")]
    pub namespace: String,
    #[serde(rename = "ts")]
    pub timestamp: DateTime,
    pub response_length: Option<u32>,
    pub millis: u32,
    #[serde(default)]
    pub keys_examined: u32,
    #[serde(default)]
    pub docs_examined: u32,
    #[serde(default)]
    pub has_sort_stage: bool,
    #[serde(default)]
    pub used_disk: bool,
    #[serde(default, rename = "ndeleted")]
    pub num_deleted: u32,
    #[serde(default, rename = "ninserted")]
    pub num_inserted: u32,
    #[serde(default, rename = "nMatched")]
    pub num_matched: u32,
    #[serde(default, rename = "nModified")]
    pub num_modified: u32,
    #[serde(default, rename = "nreturned")]
    pub num_returned: u32,
    pub write_conflicts: Option<u32>,
    pub app_name: String,
}

/// The `system.profile` collection.
pub struct SystemProfileCollection {
    collection: mongodb::Collection<SystemProfileDocument>,
}

impl MongoDbCollection for SystemProfileCollection {
    const NAME: &'static str = "system.profile";
    type Document = SystemProfileDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

impl SystemProfileCollection {
    /// Get the latest slow queries after a given timestamp.
    pub async fn get_latest_slow_queries(
        &self,
        start_timestamp: DateTime,
    ) -> Result<impl Stream<Item = Result<SystemProfileDocument, Error>>, Error> {
        self.aggregate(
            vec![
                doc! { "$match": { "ts": { "$gt": start_timestamp } } },
                doc! { "$sort": { "ts": 1 } },
            ],
            None,
        )
        .await
    }
}

/// A filter for the profiler.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct ProfileFilter {
    pub app_name: String,
    pub slow_millis: u32,
}

impl From<ProfileFilter> for mongodb::bson::Bson {
    fn from(filter: ProfileFilter) -> Self {
        Self::Document(doc! {
            "appName": filter.app_name,
            "millis": { "$gt": filter.slow_millis }
        })
    }
}
