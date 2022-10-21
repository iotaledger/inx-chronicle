// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        mongodb::{MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    types::node::{BaseToken, NodeConfiguration},
};

/// The corresponding MongoDb document representation to store [`NodeConfiguration`]s.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeConfigurationDocument {
    pub base_token: BaseToken,
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
        if let Some(latest_config) = self.get_latest_node_configuration().await? {
            if latest_config.base_token != config.base_token {
                self.replace_one(
                    doc! {},
                    NodeConfigurationDocument {
                        base_token: config.base_token,
                    },
                    None,
                )
                .await?;
            }
        } else {
            self.insert_one(
                NodeConfigurationDocument {
                    base_token: config.base_token,
                },
                None,
            )
            .await?;
        }
        Ok(())
    }

    /// Returns the latest node configuration known to Chronicle.
    pub async fn get_latest_node_configuration(&self) -> Result<Option<NodeConfigurationDocument>, Error> {
        self.find_one(doc! {}, None).await
    }
}
