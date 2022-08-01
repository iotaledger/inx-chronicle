// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{bson::doc, error::Error, options::UpdateOptions};
use serde::{Deserialize, Serialize};

use crate::{db::MongoDb, types::tangle::MilestoneIndex};

/// Provides the information about the status of the node.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct StatusDocument {
    ledger_index: Option<MilestoneIndex>,
}

impl StatusDocument {
    /// The status collection name.
    const COLLECTION: &'static str = "stardust_status";
}

impl MongoDb {
    /// Get the current ledger index.
    pub async fn get_ledger_index(&self) -> Result<Option<MilestoneIndex>, Error> {
        self.db
            .collection::<StatusDocument>(StatusDocument::COLLECTION)
            .find_one(doc! {}, None)
            .await
            .map(|doc| doc.and_then(|doc| doc.ledger_index))
    }

    /// Sets the current ledger index if it is greater than the current one.
    pub async fn update_ledger_index(&self, ledger_index: MilestoneIndex) -> Result<(), Error> {
        self.db
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
