// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use iota_sdk::types::block::{protocol::ProtocolParameters, slot::SlotIndex};
use mongodb::{bson::doc, options::UpdateOptions};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        mongodb::{DbError, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::{node::NodeConfiguration, SerializeToBson},
};

/// The MongoDb document representation of singleton Application State.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplicationStateDocument {
    pub starting_slot: Option<SlotIndex>,
    pub last_migration: Option<MigrationVersion>,
    pub node_config: Option<NodeConfiguration>,
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
    /// Gets the application starting slot index.
    pub async fn get_starting_index(&self) -> Result<Option<SlotIndex>, DbError> {
        Ok(self
            .find_one::<ApplicationStateDocument>(doc! {}, None)
            .await?
            .and_then(|doc| doc.starting_slot))
    }

    /// Set the starting slot index in the singleton application state.
    pub async fn set_starting_index(&self, starting_slot: SlotIndex) -> Result<(), DbError> {
        self.update_one(
            doc! {},
            doc! {
                "$set": { "starting_slot": starting_slot.0 }
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }

    /// Gets the last migration version of the database.
    pub async fn get_last_migration(&self) -> Result<Option<MigrationVersion>, DbError> {
        Ok(self
            .find_one::<ApplicationStateDocument>(doc! {}, None)
            .await?
            .and_then(|doc| doc.last_migration))
    }

    /// Set the current version in the singleton application state.
    pub async fn set_last_migration(&self, last_migration: MigrationVersion) -> Result<(), DbError> {
        self.update_one(
            doc! {},
            doc! {
                "$set": { "last_migration": last_migration.to_bson() }
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }

    /// Gets the node config.
    pub async fn get_node_config(&self) -> Result<Option<NodeConfiguration>, DbError> {
        Ok(self
            .find_one::<ApplicationStateDocument>(doc! {}, None)
            .await?
            .and_then(|doc| doc.node_config))
    }

    /// Set the node_config in the singleton application state.
    pub async fn set_node_config(&self, node_config: &NodeConfiguration) -> Result<(), DbError> {
        self.update_one(
            doc! {},
            doc! {
                "$set": { "node_config": node_config.to_bson() }
            },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }

    /// Gets the protocol parameters.
    pub async fn get_protocol_parameters(&self) -> Result<Option<ProtocolParameters>, DbError> {
        Ok(self
            .aggregate::<crate::model::protocol::ProtocolParameters>(
                [doc! { "$replaceWith": { "$last": "$node_config.protocol_parameters" } }],
                None,
            )
            .await?
            .try_next()
            .await?
            .map(|p| p.parameters))
    }
}
