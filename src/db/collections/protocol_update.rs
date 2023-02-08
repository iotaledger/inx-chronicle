// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::doc,
    error::Error,
    options::{FindOneOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{
        mongodb::{MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    types::tangle::{MilestoneIndex, ProtocolParameters},
};

/// A milestone's metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProtocolUpdateDocument {
    #[serde(rename = "_id")]
    pub tangle_index: MilestoneIndex,
    pub parameters: ProtocolParameters,
}

/// The stardust protocol parameters collection.
pub struct ProtocolUpdateCollection {
    collection: mongodb::Collection<ProtocolUpdateDocument>,
}

impl MongoDbCollection for ProtocolUpdateCollection {
    const NAME: &'static str = "stardust_protocol_updates";
    type Document = ProtocolUpdateDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }
}

impl ProtocolUpdateCollection {
    /// Gets the latest protocol parameters.
    pub async fn get_latest_protocol_parameters(&self) -> Result<Option<ProtocolUpdateDocument>, Error> {
        self.find_one(doc! {}, FindOneOptions::builder().sort(doc! { "_id": -1 }).build())
            .await
    }

    /// Gets the protocol parameters that are valid for the given ledger index.
    pub async fn get_protocol_parameters_for_ledger_index(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<Option<ProtocolUpdateDocument>, Error> {
        self.find_one(
            doc! { "_id": { "$lte": ledger_index } },
            FindOneOptions::builder().sort(doc! { "_id": -1 }).build(),
        )
        .await
    }

    /// Gets the protocol parameters for the given milestone index, if they were changed.
    pub async fn get_protocol_parameters_for_milestone_index(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<Option<ProtocolUpdateDocument>, Error> {
        self.find_one(doc! { "_id": milestone_index }, None).await
    }

    /// Gets the protocol parameters for a given protocol version.
    pub async fn get_protocol_parameters_for_version(
        &self,
        version: u8,
    ) -> Result<Option<ProtocolUpdateDocument>, Error> {
        self.find_one(doc! { "parameters.version": version as i32 }, None).await
    }

    /// Add the protocol parameters to the list if the protocol parameters have changed.
    pub async fn upsert_protocol_parameters(
        &self,
        ledger_index: MilestoneIndex,
        parameters: ProtocolParameters,
    ) -> Result<(), Error> {
        let params = self.get_protocol_parameters_for_ledger_index(ledger_index).await?;
        if !matches!(params, Some(params) if params.parameters == parameters) {
            self.update_one(
                doc! { "_id": ledger_index },
                doc! { "$set": {
                    "parameters": mongodb::bson::to_bson(&parameters)?
                } },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;
        }
        Ok(())
    }
}
