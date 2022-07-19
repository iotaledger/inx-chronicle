// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod alias;
mod basic;
mod foundry;
mod nft;
mod queries;

use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc},
    error::Error,
    options::IndexOptions,
    IndexModel,
};
use serde::Deserialize;

pub use self::{
    alias::AliasOutputsQuery, basic::BasicOutputsQuery, foundry::FoundryOutputsQuery, nft::NftOutputsQuery,
};
use super::OutputDocument;
use crate::{
    db::{collections::SortOrder, MongoDb},
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
    /// Gets any indexed output kind that match the provided query.
    pub async fn get_indexed_outputs<Q>(
        &self,
        query: Q,
        page_size: usize,
        cursor: Option<(MilestoneIndex, OutputId)>,
        order: SortOrder,
    ) -> Result<Option<OutputsResult>, Error>
    where
        bson::Document: From<Q>,
    {
        let ledger_index = self.get_ledger_index().await?;
        if let Some(ledger_index) = ledger_index {
            let (sort, cmp1, cmp2) = match order {
                SortOrder::Newest => (
                    doc! { "metadata.booked.milestone_index": -1, "output_id": -1 },
                    "$lt",
                    "$lte",
                ),
                SortOrder::Oldest => (
                    doc! { "metadata.booked.milestone_index": 1, "output_id": 1 },
                    "$gt",
                    "$gte",
                ),
            };

            let mut query_doc = bson::Document::from(query);
            let mut additional_queries = vec![doc! { "metadata.booked.milestone_index": { "$lte": ledger_index } }];
            if let Some((start_ms, start_output_id)) = cursor {
                additional_queries.push(doc! { "$or": [
                    doc! { "metadata.booked.milestone_index": { cmp1: start_ms } },
                    doc! {
                        "metadata.booked.milestone_index": start_ms,
                        "output_id": { cmp2: start_output_id }
                    },
                ] });
            }
            query_doc.insert("$and", additional_queries);
            let match_doc = doc! { "$match": query_doc };
            let outputs = self
                .0
                .collection::<OutputResult>(OutputDocument::COLLECTION)
                .aggregate(
                    vec![
                        match_doc,
                        doc! { "$sort": sort },
                        doc! { "$limit": page_size as i64 },
                        doc! { "$replaceWith": {
                            "output_id": "$output_id",
                            "booked_index": "$metadata.booked.milestone_index"
                        } },
                    ],
                    None,
                )
                .await?
                .map_ok(|doc| bson::from_document::<OutputResult>(doc).unwrap())
                .try_collect::<Vec<_>>()
                .await?;
            Ok(Some(OutputsResult { ledger_index, outputs }))
        } else {
            Ok(None)
        }
    }

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
                    .keys(doc! { "output.foundry_id": 1 })
                    .options(
                        IndexOptions::builder()
                            .name("output_foundry_id_index".to_string())
                            .partial_filter_expression(
                                doc! { "output.foundry_id": { "$exists": true }, "metadata.spent": null },
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
                    .keys(doc! { "output.nft_id": 1 })
                    .options(
                        IndexOptions::builder()
                            .name("output_nft_id_index".to_string())
                            .partial_filter_expression(
                                doc! { "output.nft_id": { "$exists": true }, "metadata.spent": null },
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
