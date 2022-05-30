// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::{self, doc, to_bson, to_document},
    error::Error,
    options::{FindOneOptions, UpdateOptions},
};
use serde::{Deserialize, Serialize};

use crate::{
    db::MongoDb,
    types::{
        ledger::OutputMetadata,
        stardust::block::{Output, OutputId},
    },
};

/// Contains all informations related to an output.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct OutputDocument {
    #[serde(rename = "_id")]
    output_id: OutputId,
    output: Output,
    metadata: OutputMetadata,
}

impl OutputDocument {
    /// The stardust outputs collection name.
    const COLLECTION: &'static str = "stardust_outputs";
}

/// Queries that are related to [`Output`]s
impl MongoDb {
    /// Upserts a [`Output`] together with its associated [`OutputMetadata`].
    pub(super) async fn upsert_output_with_metadata(
        &self,
        output_id: OutputId,
        output: Output,
        metadata: OutputMetadata,
    ) -> Result<(), Error> {
        let output_document = OutputDocument {
            output_id: output_id.clone(),
            output,
            metadata,
        };

        self.0
            .collection::<OutputDocument>(OutputDocument::COLLECTION)
            .update_one(
                doc! { "_id": to_bson(&output_id)?},
                doc! {"$set": to_document(&output_document)? },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }

    /// Get an [`Output`] by [`OutputId`].
    pub async fn get_output(&self, output_id: &OutputId) -> Result<Option<Output>, Error> {
        self.0
            .collection::<Output>(OutputDocument::COLLECTION)
            .find_one(
                doc! {"_id": bson::to_bson(output_id)?},
                Some(FindOneOptions::builder().projection(doc! {"output": 1 }).build()),
            )
            .await
    }

    /// Get an [`Output`] together with its [`OutputMetadata`] by [`OutputId`].
    pub async fn get_output_and_metadata(
        &self,
        output_id: &OutputId,
    ) -> Result<Option<(Output, OutputMetadata)>, Error> {
        // TODO make this one query!
        let maybe_output = self.get_output(output_id).await?;
        let maybe_metadata = self.get_output_metadata(output_id).await?;

        let combined = match (maybe_output, maybe_metadata) {
            (Some(output), Some(metadata)) => Some((output, metadata)),
            _ => None,
        };

        Ok(combined)
    }

    /// Get the metadata of an [`Output`] by its [`OutputId`].
    pub async fn get_output_metadata(&self, output_id: &OutputId) -> Result<Option<OutputMetadata>, Error> {
        self.0
            .collection::<OutputMetadata>(OutputDocument::COLLECTION)
            .find_one(
                doc! {"_id": bson::to_bson(output_id)?},
                Some(FindOneOptions::builder().projection(doc! {"metadata": 1 }).build()),
            )
            .await
    }
}
