// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::{
    bson::{self, doc},
    error::Error,
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
pub(crate) struct OutputDocument {
    #[serde(rename = "_id")]
    output_id: OutputId,
    output: Output,
    metadata: OutputMetadata,
}

impl OutputDocument {
    /// The stardust outputs collection name.
    pub(crate) const COLLECTION: &'static str = "stardust_outputs";
}

/// # Queries that are related to [`Output`]s
impl MongoDb {
    /// Upserts a [`Output`] together with its associated [`OutputMetadata`].
    pub async fn upsert_output_update(
        &self,
        _output_id: OutputId,
        _output: Output,
        _metadata: OutputMetadata,
    ) -> Result<(), Error> {
        // let block_document = BlockDocument {
        //     block_id,
        //     block,
        //     raw,
        //     metadata: Some(metadata),
        // };

        // let _ = self
        //     .0
        //     .collection::<BlockDocument>(BlockDocument::COLLECTION)
        //     .insert_one(block_document, None)
        //     .await;

        // Ok(())
        todo!()
    }

    /// Get an [`Output`] by [`OutputId`].
    pub async fn get_output(&self, output_id: &OutputId) -> Result<Option<Output>, Error> {
        self.0
            .collection::<Output>(OutputDocument::COLLECTION)
            .find_one(doc! {"_id": bson::to_bson(output_id)?}, Self::projection("output"))
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
            .find_one(doc! {"_id": bson::to_bson(output_id)?}, Self::projection("metadata"))
            .await
    }
}
