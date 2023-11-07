// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::slot::SlotIndex;
use mongodb::{
    bson::doc,
    options::{FindOneOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        mongodb::{DbError, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    inx::responses::NodeConfiguration,
    model::SerializeToBson,
};

/// The corresponding MongoDb document representation to store [`NodeConfiguration`]s.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationUpdateDocument {
    #[serde(rename = "_id")]
    pub slot_index: SlotIndex,
    #[serde(flatten)]
    pub config: NodeConfiguration,
}

/// A collection to store [`NodeConfiguration`]s.
pub struct ConfigurationUpdateCollection {
    collection: mongodb::Collection<ConfigurationUpdateDocument>,
}

impl MongoDbCollection for ConfigurationUpdateCollection {
    const NAME: &'static str = "iota_configuration_updates";
    type Document = ConfigurationUpdateDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

impl ConfigurationUpdateCollection {
    /// Gets the latest node configuration.
    pub async fn get_latest_node_configuration(&self) -> Result<Option<ConfigurationUpdateDocument>, DbError> {
        Ok(self
            .find_one(doc! {}, FindOneOptions::builder().sort(doc! { "_id": -1 }).build())
            .await?)
    }

    /// Gets the node configuration that was valid for the given slot index.
    pub async fn get_node_configuration_for_slot_index(
        &self,
        slot_index: SlotIndex,
    ) -> Result<Option<ConfigurationUpdateDocument>, DbError> {
        Ok(self
            .find_one(
                doc! { "_id": { "$lte": slot_index.0 } },
                FindOneOptions::builder().sort(doc! { "_id": -1 }).build(),
            )
            .await?)
    }

    /// Inserts or updates a node configuration for a given slot index.
    pub async fn upsert_node_configuration(
        &self,
        slot_index: SlotIndex,
        config: NodeConfiguration,
    ) -> Result<(), DbError> {
        let node_config = self.get_node_configuration_for_slot_index(slot_index).await?;
        if !matches!(node_config, Some(node_config) if node_config.config == config) {
            self.update_one(
                doc! { "_id": slot_index.0 },
                doc! { "$set": config.to_bson() },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;
        }
        Ok(())
    }
}
