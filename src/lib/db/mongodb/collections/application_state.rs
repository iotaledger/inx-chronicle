// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{bson::doc, error::Error, options::UpdateOptions};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        mongodb::{MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::block::payload::milestone::MilestoneIndexTimestamp,
};

/// The MongoDb document representation of singleton Application State.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplicationStateDocument {
    pub starting_index: Option<MilestoneIndexTimestamp>,
    pub last_migration: Option<MigrationVersion>,
}

/// The migration version and associated metadata.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MigrationVersion {
    pub id: usize,
    pub app_version: String,
    pub date: time::Date,
}

impl std::fmt::Display for MigrationVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {} - {}", self.id, self.app_version, self.date)
    }
}

/// A collection to store singleton Application State.
pub struct ApplicationStateCollection {
    collection: mongodb::Collection<ApplicationStateDocument>,
}

impl MongoDbCollection for ApplicationStateCollection {
    const NAME: &'static str = "application_state";
    type Document = ApplicationStateDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

impl ApplicationStateCollection {
    /// Gets the application starting milestone index.
    pub async fn get_starting_index(&self) -> Result<Option<MilestoneIndexTimestamp>, Error> {
        Ok(self
            .find_one::<ApplicationStateDocument>(doc! {}, None)
            .await?
            .and_then(|doc| doc.starting_index))
    }

    /// Set the starting milestone index in the singleton application state.
    pub async fn set_starting_index(&self, starting_index: MilestoneIndexTimestamp) -> Result<(), Error> {
        self.update_one(
            doc! {},
            doc! {
                "$set": { "starting_index": starting_index }
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }

    /// Gets the last migration version of the database.
    pub async fn get_last_migration(&self) -> Result<Option<MigrationVersion>, Error> {
        Ok(self
            .find_one::<ApplicationStateDocument>(doc! {}, None)
            .await?
            .and_then(|doc| doc.last_migration))
    }

    /// Set the current version in the singleton application state.
    pub async fn set_last_migration(&self, last_migration: MigrationVersion) -> Result<(), Error> {
        self.update_one(
            doc! {},
            doc! {
                "$set": { "last_migration": mongodb::bson::to_bson(&last_migration)? }
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }
}
