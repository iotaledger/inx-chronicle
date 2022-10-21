// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        mongodb::{MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    types::node::NodeConfiguration,
};

/// The corresponding MongoDb document representation to store [`NodeConfiguration`]s.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeConfigurationDocument(NodeConfiguration);

/// A single-document collection to store the latest [`NodeConfiguration`].
pub struct NodeConfigurationCollection {
    collection: mongodb::Collection<NodeConfigurationDocument>,
}

impl MongoDbCollection for NodeConfigurationCollection {
    const NAME: &'static str = "node_configuration";
    type Document = NodeConfigurationDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

impl NodeConfigurationCollection {
    /// Updates the stored node configuration - if necessary.
    pub async fn update_node_configuration(&self, config: NodeConfiguration) -> Result<(), Error> {
        if let Some(latest_config) = self.get_latest_node_configuration().await? {
            if latest_config != config {
                self.replace_one(doc! {}, NodeConfigurationDocument(config), None)
                    .await?;
            }
        } else {
            self.insert_one(NodeConfigurationDocument(config), None).await?;
        }
        Ok(())
    }

    /// Returns the latest node configuration known to Chronicle.
    pub async fn get_latest_node_configuration(&self) -> Result<Option<NodeConfiguration>, Error> {
        Ok(self
            .find_one(doc! {}, None)
            .await?
            .map(|node_configuration: NodeConfigurationDocument| node_configuration.0))
    }
}
