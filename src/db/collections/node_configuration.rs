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
pub struct NodeConfigurationDocument {
    #[serde(rename = "_id")]
    id: (),
    pub config: NodeConfiguration,
}

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
        if !matches!(self.get_latest_node_configuration().await?, Some(latest_config) if latest_config.config == config)
        {
            self.insert_one(NodeConfigurationDocument { id: (), config }, None)
                .await?;
        }
        Ok(())
    }

    /// Returns the latest node configuration known to Chronicle.
    pub async fn get_latest_node_configuration(&self) -> Result<Option<NodeConfigurationDocument>, Error> {
        self.find_one(doc! {}, None).await
    }
}
