// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod alias;
mod basic;
mod foundry;
mod nft;
mod queries;

use derive_more::From;
use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc, Bson},
    error::Error,
    options::IndexOptions,
    IndexModel,
};
use serde::Deserialize;

pub use self::{
    alias::AliasOutputsQuery, basic::BasicOutputsQuery, foundry::FoundryOutputsQuery, nft::NftOutputsQuery,
};
use super::{OutputCollection, OutputDocument};
use crate::{
    db::{collections::SortOrder, mongodb::MongoDbCollectionExt},
    types::{
        ledger::OutputMetadata,
        stardust::block::output::{AliasId, FoundryId, NftId, OutputId},
        tangle::MilestoneIndex,
    },
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
    pub outputs: Vec<OutputResult>,
}

#[derive(From)]
#[allow(missing_docs)]
pub enum IndexedId {
    Alias(AliasId),
    Foundry(FoundryId),
    Nft(NftId),
}

impl From<IndexedId> for Bson {
    fn from(id: IndexedId) -> Self {
        match id {
            IndexedId::Alias(id) => id.into(),
            IndexedId::Foundry(id) => id.into(),
            IndexedId::Nft(id) => id.into(),
        }
    }
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct IndexedOutputResult {
    pub output_id: OutputId,
}

impl OutputCollection {
    /// Gets the current unspent indexed output id with the given indexed id.
    pub async fn get_indexed_output_by_id(
        &self,
        id: impl Into<IndexedId>,
        ledger_index: MilestoneIndex,
    ) -> Result<Option<IndexedOutputResult>, Error> {
        let id = id.into();
        let id_string = match id {
            IndexedId::Alias(_) => "output.alias_id",
            IndexedId::Foundry(_) => "output.foundry_id",
            IndexedId::Nft(_) => "output.nft_id",
        };
        let mut res = self
            .aggregate(
                vec![
                    doc! { "$match": {
                        id_string: id,
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                    } },
                    doc! { "$sort": { "metadata.booked.milestone_index": -1 } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?;
        if let Some(OutputDocument {
            metadata: OutputMetadata {
                spent_metadata: spent @ Some(_),
                ..
            },
            ..
        }) = res.as_mut()
        {
            // TODO: record that we got an output that is spent past the ledger_index to metrics
            spent.take();
        }
        Ok(res.map(|doc| IndexedOutputResult {
            output_id: doc.output_id,
        }))
    }

    /// Gets any indexed output kind that match the provided query.
    pub async fn get_indexed_outputs<Q>(
        &self,
        query: Q,
        page_size: usize,
        cursor: Option<(MilestoneIndex, OutputId)>,
        order: SortOrder,
        include_spent: bool,
        ledger_index: MilestoneIndex,
    ) -> Result<OutputsResult, Error>
    where
        bson::Document: From<Q>,
    {
        let (sort, cmp1, cmp2) = match order {
            SortOrder::Newest => (doc! { "metadata.booked.milestone_index": -1, "_id": -1 }, "$lt", "$lte"),
            SortOrder::Oldest => (doc! { "metadata.booked.milestone_index": 1, "_id": 1 }, "$gt", "$gte"),
        };

        let query_doc = bson::Document::from(query);
        let mut additional_queries = vec![doc! { "metadata.booked.milestone_index": { "$lte": ledger_index } }];
        if !include_spent {
            additional_queries.push(doc! {
                "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
            });
        }
        if let Some((start_ms, start_output_id)) = cursor {
            additional_queries.push(doc! { "$or": [
                doc! { "metadata.booked.milestone_index": { cmp1: start_ms } },
                doc! {
                    "metadata.booked.milestone_index": start_ms,
                    "_id": { cmp2: start_output_id }
                },
            ] });
        }
        let match_doc = doc! { "$match": {
            "$and": [
                query_doc,
                { "$and": additional_queries }
            ]
        } };
        let outputs = self
            .aggregate(
                vec![
                    match_doc,
                    doc! { "$sort": sort },
                    doc! { "$limit": page_size as i64 },
                    doc! { "$replaceWith": {
                        "output_id": "$_id",
                        "booked_index": "$metadata.booked.milestone_index"
                    } },
                ],
                None,
            )
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        Ok(OutputsResult { outputs })
    }

    /// Creates indexer output indexes.
    pub async fn create_indexer_indexes(&self) -> Result<(), Error> {
        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.kind": 1 })
                .options(IndexOptions::builder().name("output_kind_index".to_string()).build())
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.alias_id": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_alias_id_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.alias_id": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.foundry_id": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_foundry_id_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.foundry_id": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.nft_id": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_nft_id_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.nft_id": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.address_unlock_condition": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_address_unlock_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.address_unlock_condition": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.storage_deposit_return_unlock_condition": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_storage_deposit_return_unlock_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.storage_deposit_return_unlock_condition": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.timelock_unlock_condition": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_timelock_unlock_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.timelock_unlock_condition": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.expiration_unlock_condition": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_expiration_unlock_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.expiration_unlock_condition": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.state_controller_address_unlock_condition": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_state_controller_unlock_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.state_controller_address_unlock_condition": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.governor_address_unlock_condition": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_governor_address_unlock_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.governor_address_unlock_condition": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.immutable_alias_address_unlock_condition": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_immutable_alias_address_unlock_index".to_string())
                        .partial_filter_expression(doc! {
                            "output.immutable_alias_address_unlock_condition": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.features": 1 })
                .options(IndexOptions::builder().name("output_feature_index".to_string()).build())
                .build(),
            None,
        )
        .await?;

        self.create_index(
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

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "metadata.booked.milestone_index": -1 })
                .options(
                    IndexOptions::builder()
                        .name("output_booked_milestone_index".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(
                    doc! { "metadata.spent_metadata.spent.milestone_index": -1, "metadata.booked.milestone_index": -1,  "details.address": 1 },
                )
                .options(
                    IndexOptions::builder()
                        .name("output_spent_milestone_index_comp".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}
