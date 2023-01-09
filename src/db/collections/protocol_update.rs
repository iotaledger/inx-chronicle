use mongodb::{bson::doc, error::Error, options::FindOneOptions};
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

    /// Inserts a protocol parameters for a given milestone index.
    pub async fn insert_protocol_parameters(
        &self,
        tangle_index: MilestoneIndex,
        parameters: ProtocolParameters,
    ) -> Result<(), Error> {
        self.insert_one(
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
