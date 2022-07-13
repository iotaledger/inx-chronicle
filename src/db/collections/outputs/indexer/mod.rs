// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod alias;
mod basic;

use mongodb::{bson::doc, error::Error, options::IndexOptions, IndexModel};
use serde::Deserialize;

pub use self::{alias::AliasOutputsQuery, basic::BasicOutputsQuery};
use super::OutputDocument;
use crate::{
    db::MongoDb,
    types::{stardust::block::OutputId, tangle::MilestoneIndex},
};

#[derive(Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct OutputResult {
    pub output_id: OutputId,
    pub booked_index: MilestoneIndex,
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct OutputsResult {
    pub ledger_index: MilestoneIndex,
    pub outputs: Vec<OutputResult>,
}

impl MongoDb {
    /// Creates indexer output indexes.
    pub async fn create_indexer_output_indexes(&self) -> Result<(), Error> {
        let collection = self.0.collection::<OutputDocument>(OutputDocument::COLLECTION);

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "output.kind": 1 })
                    .options(IndexOptions::builder().name("output_kind_index".to_string()).build())
                    .build(),
                None,
            )
            .await?;

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "output.alias_id": 1 })
                    .options(
                        IndexOptions::builder()
                            .name("output_alias_id_index".to_string())
                            .partial_filter_expression(
                                doc! { "output.alias_id": { "$exists": true }, "metadata.spent": null },
                            )
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "output.unlock_conditions": 1 })
                    .options(IndexOptions::builder().name("output_unlock_index".to_string()).build())
                    .build(),
                None,
            )
            .await?;

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "output.features": 1 })
                    .options(IndexOptions::builder().name("output_feature_index".to_string()).build())
                    .build(),
                None,
            )
            .await?;

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "output.native_tokens": 1 })
                    .options(
                        IndexOptions::builder()
                            .name("output_native_tokens_index".to_string())
                            .build(),
                    )
                    .build(),
                None,
            )
            .await?;

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "metadata.booked": -1 })
                    .options(IndexOptions::builder().name("output_booked_index".to_string()).build())
                    .build(),
                None,
            )
            .await?;

        Ok(())
    }
}
