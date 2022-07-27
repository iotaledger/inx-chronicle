// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::{doc, to_document},
    error::Error,
    options::UpdateOptions,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::tangle::{MilestoneIndex, ProtocolInfo},
};

/// Provides the information about the status of the node.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct StatusDocument {
    protocol: Option<ProtocolInfo>,
    ledger_index: Option<MilestoneIndex>,
}

impl StatusDocument {
    /// The status collection name.
    const COLLECTION: &'static str = "stardust_status";
}

impl MongoDb {
    /// Get the name of the network.
    pub async fn get_protocol_parameters(&self) -> Result<Option<ProtocolInfo>, Error> {
        self.0
            .collection::<StatusDocument>(StatusDocument::COLLECTION)
            .find_one(doc! {}, None)
            .await
            .map(|doc| doc.and_then(|doc| doc.protocol))
    }

    /// Sets the name of the network.
    pub async fn set_protocol_parameters(&self, protocol_info: ProtocolInfo) -> Result<(), Error> {
        self.0
            .collection::<StatusDocument>(StatusDocument::COLLECTION)
            .update_one(
                doc! {},
                doc! { "$set": { "protocol.parameters": to_document(&protocol_info)? } },
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
