// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use crate::db::MongoDb;

/// Provides the information about the status of the node.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct StatusDocument {
    network_name: String,
}

impl StatusDocument {
    /// The status collection name.
    const COLLECTION: &'static str = "status";
}

impl MongoDb {
    /// Get the name of the network.
    pub async fn get_network_name(&self) -> Result<Option<String>, Error> {
        self.db
            .collection::<StatusDocument>(StatusDocument::COLLECTION)
            .find_one(doc! {}, None)
            .await
            .map(|doc| doc.map(|doc| doc.network_name))
    }

    /// Sets the name of the network.
    pub async fn set_network_name(&self, network_name: String) -> Result<(), Error> {
        self.db
            .collection::<StatusDocument>(StatusDocument::COLLECTION)
            .insert_one(StatusDocument { network_name }, None)
            .await?;

        Ok(())
    }
}
