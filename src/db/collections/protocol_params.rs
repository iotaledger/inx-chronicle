// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{bson::doc, error::Error, options::FindOneOptions};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::tangle::{MilestoneIndex, ProtocolParameters},
};

/// A milestone's metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolUpdateDocument {
    #[serde(rename = "_id")]
    pub tangle_index: MilestoneIndex,
    pub parameters: ProtocolParameters,
}

impl ProtocolUpdateDocument {
    /// The stardust milestone collection name.
    const COLLECTION: &'static str = "stardust_protocol_updates";
}

impl MongoDb {
    /// Gets the latest protocol parameters.
    pub async fn get_latest_protocol_parameters(&self) -> Result<Option<ProtocolUpdateDocument>, Error> {
        Ok(self
            .0
            .collection::<ProtocolUpdateDocument>(ProtocolUpdateDocument::COLLECTION)
            .find_one(doc! {}, FindOneOptions::builder().sort(doc! { "_id": -1 }).build())
            .await?)
    }

    /// Inserts a protocol parameters for a given milestone index.
    pub async fn insert_protocol_parameters(
        &self,
        tangle_index: MilestoneIndex,
        parameters: ProtocolParameters,
    ) -> Result<(), Error> {
        self.0
            .collection::<ProtocolUpdateDocument>(ProtocolUpdateDocument::COLLECTION)
            .insert_one(
                ProtocolUpdateDocument {
                    tangle_index,
                    parameters,
                },
                None,
            )
            .await?;

        Ok(())
    }

    /// Add the protocol parameters to the list if the protocol parameters have changed.
    pub async fn update_latest_protocol_parameters(
        &self,
        tangle_index: MilestoneIndex,
        parameters: ProtocolParameters,
    ) -> Result<(), Error> {
        if let Some(latest_params) = self.get_latest_protocol_parameters().await? {
            if latest_params.parameters != parameters {
                self.insert_protocol_parameters(tangle_index, parameters).await?;
            }
        } else {
            self.insert_protocol_parameters(tangle_index, parameters).await?;
        }
        Ok(())
    }
}
