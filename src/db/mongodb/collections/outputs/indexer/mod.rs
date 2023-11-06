// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod alias;
mod basic;
mod foundry;
mod nft;
mod queries;

use derive_more::From;
use futures::TryStreamExt;
use iota_sdk::types::block::{
    output::{AccountId, AnchorId, DelegationId, FoundryId, NftId, OutputId},
    slot::SlotIndex,
};
use mongodb::{
    bson::{self, doc, Bson},
    options::IndexOptions,
    IndexModel,
};
use serde::{Deserialize, Serialize};

pub use self::{
    alias::AliasOutputsQuery, basic::BasicOutputsQuery, foundry::FoundryOutputsQuery, nft::NftOutputsQuery,
};
use super::{OutputCollection, OutputDocument};
use crate::{
    db::mongodb::{collections::SortOrder, DbError, MongoDbCollectionExt},
    model::SerializeToBson,
};

#[derive(Clone, Debug, Deserialize)]
#[allow(missing_docs)]
pub struct OutputResult {
    pub output_id: OutputId,
    pub booked_index: SlotIndex,
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct OutputsResult {
    pub outputs: Vec<OutputResult>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, From)]
#[serde(untagged)]
#[allow(missing_docs)]
pub enum IndexedId {
    Account(AccountId),
    Foundry(FoundryId),
    Nft(NftId),
    Delegation(DelegationId),
    Anchor(AnchorId),
}

impl IndexedId {
    /// Get the indexed ID kind.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Account(_) => "account",
            Self::Foundry(_) => "foundry",
            Self::Nft(_) => "nft",
            Self::Delegation(_) => "delegation",
            Self::Anchor(_) => "anchor",
        }
    }
}

impl From<IndexedId> for Bson {
    fn from(id: IndexedId) -> Self {
        match id {
            IndexedId::Account(id) => id.to_bson(),
            IndexedId::Foundry(id) => id.to_bson(),
            IndexedId::Nft(id) => id.to_bson(),
            IndexedId::Delegation(id) => id.to_bson(),
            IndexedId::Anchor(id) => id.to_bson(),
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
        ledger_index: SlotIndex,
    ) -> Result<Option<IndexedOutputResult>, DbError> {
        let id = id.into();
        let mut res = self
            .aggregate(
                [
                    doc! { "$match": {
                        "output.kind": id.kind(),
                        "details.indexed_id": id,
                        "metadata.slot_booked": { "$lte": ledger_index.0 },
                        "metadata.spent_metadata.slot_spent": { "$not": { "$lte": ledger_index.0 } }
                    } },
                    doc! { "$sort": { "metadata.slot_booked": -1 } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?;
        if let Some(OutputDocument { metadata, .. }) = res.as_mut() {
            if metadata.spent_metadata.is_some() {
                // TODO: record that we got an output that is spent past the slot index to metrics
            }
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
        cursor: Option<(SlotIndex, OutputId)>,
        order: SortOrder,
        include_spent: bool,
        ledger_index: SlotIndex,
    ) -> Result<OutputsResult, DbError>
    where
        bson::Document: From<Q>,
    {
        let (sort, cmp1, cmp2) = match order {
            SortOrder::Newest => (doc! { "metadata.slot_booked": -1, "_id": -1 }, "$lt", "$lte"),
            SortOrder::Oldest => (doc! { "metadata.slot_booked": 1, "_id": 1 }, "$gt", "$gte"),
        };

        let query_doc = bson::Document::from(query);
        let mut additional_queries = vec![doc! { "metadata.slot_booked": { "$lte": ledger_index.0 } }];
        if !include_spent {
            additional_queries.push(doc! {
                "metadata.spent_metadata.slot_spent": { "$not": { "$lte": ledger_index.0 } }
            });
        }
        if let Some((start_slot, start_output_id)) = cursor {
            additional_queries.push(doc! { "$or": [
                doc! { "metadata.slot_booked": { cmp1: start_slot.0 } },
                doc! {
                    "metadata.slot_booked": start_slot.0,
                    "_id": { cmp2: start_output_id.to_bson() }
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
                [
                    match_doc,
                    doc! { "$sort": sort },
                    doc! { "$limit": page_size as i64 },
                    doc! { "$replaceWith": {
                        "output_id": "$_id",
                        "booked_index": "$metadata.slot_booked"
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
    pub async fn create_indexer_indexes(&self) -> Result<(), DbError> {
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
                .keys(doc! { "details.indexed_id": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_indexed_id_index".to_string())
                        .partial_filter_expression(doc! {
                            "details.indexed_id": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "details.address": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_owning_address_index".to_string())
                        .partial_filter_expression(doc! {
                            "details.address": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(doc! { "output.storage_deposit_return_unlock_condition.return_address": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_storage_deposit_return_unlock_return_address_index".to_string())
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
                .keys(doc! { "output.timelock_unlock_condition.timestamp": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_timelock_unlock_timestamp_index".to_string())
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
                .keys(doc! { "output.expiration_unlock_condition.return_address": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_expiration_unlock_return_address_index".to_string())
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
                .keys(doc! { "output.expiration_unlock_condition.timestamp": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_expiration_unlock_timestamp_index".to_string())
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
                .keys(doc! { "output.governor_address_unlock_condition.address": 1 })
                .options(
                    IndexOptions::builder()
                        .name("output_governor_address_unlock_address_index".to_string())
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
                .keys(doc! { "metadata.slot_booked": -1 })
                .options(IndexOptions::builder().name("output_booked_slot".to_string()).build())
                .build(),
            None,
        )
        .await?;

        self.create_index(
            IndexModel::builder()
                .keys(
                    doc! { "metadata.spent_metadata.slot_spent": -1, "metadata.slot_booked": 1,  "details.address": 1 },
                )
                .options(
                    IndexOptions::builder()
                        .name("output_spent_slot_comp".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}
