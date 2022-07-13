// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{bson::doc, error::Error, options::UpdateOptions};
use serde::{Deserialize, Serialize};

use crate::{db::MongoDb, types::tangle::MilestoneIndex};

/// Provides the information about the status of the node.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct StatusDocument {
    network_name: Option<String>,
    ledger_index: Option<MilestoneIndex>,
}

impl StatusDocument {
    /// The status collection name.
    const COLLECTION: &'static str = "status";
}

impl MongoDb {
    /// Get the name of the network.
    #[deprecated(note = "Use `ProtocolParameterDocument` instead.")]
    pub async fn get_network_name(&self) -> Result<Option<String>, Error> {
        self.0
            .collection::<StatusDocument>(StatusDocument::COLLECTION)
            .find_one(doc! {}, None)
            .await
            .map(|doc| doc.and_then(|doc| doc.network_name))
    }

    /// Sets the name of the network.
    #[deprecated(note = "Use `ProtocolParameterDocument` instead.")]
    pub async fn set_network_name(&self, network_name: String) -> Result<(), Error> {
        self.0
            .collection::<StatusDocument>(StatusDocument::COLLECTION)
            .update_one(
                doc! {},
                doc! { "$set": { "network_name": network_name } },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    /// Get the current ledger index.
    pub async fn get_ledger_index(&self) -> Result<Option<MilestoneIndex>, Error> {
        self.0
            .collection::<StatusDocument>(StatusDocument::COLLECTION)
            .find_one(doc! {}, None)
            .await
            .map(|doc| doc.and_then(|doc| doc.ledger_index))
    }

    /// Sets the current ledger index if it is greater than the current one.
    pub async fn update_ledger_index(&self, ledger_index: MilestoneIndex) -> Result<(), Error> {
        self.0
            .collection::<StatusDocument>(StatusDocument::COLLECTION)
            .update_one(
                doc! {},
                vec![doc! { "$set": {
                    "ledger_index": { "$max": [ "$ledger_index", ledger_index ] }
                } }],
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }
}
