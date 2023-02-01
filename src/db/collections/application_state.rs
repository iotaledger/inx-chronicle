// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{bson::doc, error::Error, options::UpdateOptions};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        mongodb::{MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    types::ledger::MilestoneIndexTimestamp,
};

/// The MongoDb document representation of singleton Application State.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplicationStateDocument {
    pub starting_index: MilestoneIndexTimestamp,
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
    /// Gets the singleton application state.
    pub async fn get_application_state(&self) -> Result<Option<ApplicationStateDocument>, Error> {
        self.find_one(doc! {}, None).await
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
}
