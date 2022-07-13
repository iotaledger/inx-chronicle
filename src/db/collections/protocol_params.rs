// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::{self, doc},
    error::Error,
    options::{FindOneOptions, IndexOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        ledger::MilestoneIndexTimestamp,
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

/// Contains all information related to an output.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProtocolParametersDocument {
    /// The milestone index for which the parameters become active.
    at: MilestoneIndexTimestamp,
    parameters: ProtocolParameters,
    update: bool,
}

impl ProtocolParametersDocument {
    /// The stardust outputs collection name.
    const COLLECTION: &'static str = "stardust_protocol_parameters";
}

impl MongoDb {
    /// Creates protocol parameter indexes.
    pub async fn create_protocol_parameter_indexes(&self) -> Result<(), Error> {
        self.0
            .collection::<ProtocolParametersDocument>(ProtocolParametersDocument::COLLECTION)
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "at.milestone_index": -1, "parameters.version": -1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
                None,
            )
            .await?;

        Ok(())
    }

    /// Inserts a new set of [`ProtocolParameters`] into the database.
    pub async fn insert_protocol_parameters(
        &self,
        at: MilestoneIndexTimestamp,
        parameters: ProtocolParameters,
        update: bool,
    ) -> Result<(), Error> {
        let doc = ProtocolParametersDocument { at, parameters, update };
        self.0
            .collection::<ProtocolParametersDocument>(ProtocolParametersDocument::COLLECTION)
            .update_one(
                doc! { "at.milestone_index": at.milestone_index, "parameters.version": &doc.parameters.version },
                doc! { "$set": bson::to_document(&doc)? },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    /// Find the latest milestone inserted.
    pub async fn get_protocol_parameters(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<Option<ProtocolParameters>, Error> {
        Ok(self
            .0
            .collection::<ProtocolParametersDocument>(ProtocolParametersDocument::COLLECTION)
            .find_one(
                doc! { "at.milestone_index": milestone_index },
                FindOneOptions::builder()
                    .sort(doc! {"at.milestone_index": -1, "parameters.version": -1 })
                    .build(),
            )
            .await?
            .map(|d| d.parameters))
    }
}
