// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{protocol::ProtocolParameters, slot::EpochIndex};
use mongodb::{
    bson::doc,
    error::Error,
    options::{FindOneOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};

use crate::db::{
    mongodb::{MongoDbCollection, MongoDbCollectionExt},
    MongoDb,
};

/// A milestone's metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProtocolUpdateDocument {
    #[serde(rename = "_id")]
    pub start_epoch: EpochIndex,
    pub parameters: ProtocolParameters,
}

/// The iota protocol parameters collection.
pub struct ProtocolUpdateCollection {
    collection: mongodb::Collection<ProtocolUpdateDocument>,
}

impl MongoDbCollection for ProtocolUpdateCollection {
    const NAME: &'static str = "iota_protocol_updates";
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
    pub async fn get_protocol_parameters_for_epoch_index(
        &self,
        epoch_index: EpochIndex,
    ) -> Result<Option<ProtocolUpdateDocument>, Error> {
        self.find_one(
            doc! { "_id": { "$lte": epoch_index.0 } },
            FindOneOptions::builder().sort(doc! { "_id": -1 }).build(),
        )
        .await
    }

    /// Gets the protocol parameters for the given milestone index, if they were changed.
    pub async fn get_protocol_parameters_for_milestone_index(
        &self,
        epoch_index: EpochIndex,
    ) -> Result<Option<ProtocolUpdateDocument>, Error> {
        self.find_one(doc! { "_id": epoch_index.0 }, None).await
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
        epoch_index: EpochIndex,
        parameters: ProtocolParameters,
    ) -> Result<(), Error> {
        let params = self.get_protocol_parameters_for_epoch_index(epoch_index).await?;
        if !matches!(params, Some(params) if params.parameters == parameters) {
            self.update_one(
                doc! { "_id": epoch_index.0 },
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
