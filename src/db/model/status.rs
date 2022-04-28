// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::{self, doc},
    error::Error,
    options::UpdateOptions,
    results::UpdateResult,
};
use serde::{Deserialize, Serialize};

use super::collection;
use crate::db::MongoDb;

/// Provides the information about the status of the node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
    network_name: String,
}

impl MongoDb {
    /// Get the persistet [`Status`] from the database.
    pub async fn status(&self) -> Result<Option<Status>, Error> {
        self.0
            .collection::<Status>(collection::STATUS)
            .find_one(doc! {}, None)
            .await
    }

    /// Upserts a [`Status`] to the database.
    pub async fn upsert_status(&self, status: &Status) -> Result<UpdateResult, Error> {
        self.0
            .collection::<Status>(collection::STATUS)
            .update_one(
                doc! {},
                doc! {"$set": bson::to_document(status)?},
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
    }
}
