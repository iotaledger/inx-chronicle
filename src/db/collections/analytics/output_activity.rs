// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::Error;
use crate::{
    db::{collections::OutputCollection, MongoDbCollectionExt},
    types::{
        stardust::block::output::{AliasId, NftId},
        tangle::MilestoneIndex,
    },
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(default)]
pub struct AliasActivityAnalyticsResult {
    pub created_count: u64,
    pub governor_changed_count: u64,
    pub state_changed_count: u64,
    pub destroyed_count: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(default)]
pub struct NftActivityAnalyticsResult {
    pub created_count: u64,
    pub transferred_count: u64,
    pub destroyed_count: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[serde(default)]
pub struct OutputActivityAnalyticsResult {
    pub alias: AliasActivityAnalyticsResult,
    pub nft: NftActivityAnalyticsResult,
}

impl OutputCollection {
    /// Gathers analytics about nft outputs that were created/transferred/burned in the given milestone.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_nft_output_activity_analytics(
        &self,
        index: MilestoneIndex,
    ) -> Result<NftActivityAnalyticsResult, Error> {
        Ok(self
            .aggregate(
                vec![
                    // Match all nft outputs in the given milestone.
                    doc! { "$match": {
                        "$and": [
                            { "$or": [
                                { "metadata.booked.milestone_index": index },
                                { "metadata.spent_metadata.spent.milestone_index": index },
                            ] },
                            { "output.kind": "nft" },
                        ]
                    } },
                    // Screen outputs for being booked and/or spent. An output that was booked and
                    // spent will appear in both arrays, but with different ids.
                    doc! { "$facet": {
                        "booked_screening": [
                            { "$project": {
                                "_id": {
                                    "$cond": [
                                        { "$eq": [ "$metadata.booked.milestone_index", index ] },
                                        "$_id.transaction_id",
                                        "$metadata.spent_metadata.transaction_id"
                                    ]
                                },
                                "output_id": "$_id",
                                "asset_id": "$output.nft_id",
                            } }
                        ],
                        "spent_screening": [
                            { "$project": {
                                "_id": {
                                    "$cond": [
                                        { "$eq": [ "$metadata.spent_metadata.spent.milestone_index", index ] },
                                        "$metadata.spent_metadata.transaction_id",
                                        "$_id.transaction_id"
                                    ]
                                },
                                "output_id": "$_id",
                                "asset_id": "$output.nft_id",
                                }
                            }
                        ]
                    } },
                    // Merge both arrays from the previous facet operation. That will remove duplicates (outputs
                    // where each screening produced the same result) and keep outputs that were booked and spent
                    // within the same milestone. 
                    doc! { "$project": { "nft_outputs": { "$setUnion": [ "$booked_screening", "$spent_screening" ] } } },
                    doc! { "$unwind": { "path": "$nft_outputs" } },
                    // Reconstruct the inputs and outputs for the given asset.
                    doc! { "$group": {
                        "_id": "$nft_outputs._id",
                        "inputs": { "$push": {
                            "$cond": [
                                { "$ne": [ "$nft_outputs.output_id.transaction_id", "$nft_outputs._id" ] },
                                {
                                    "id": "$nft_outputs.output_id",
                                    "asset_id": "$nft_outputs.asset_id"
                                },
                                null
                            ]
                        } },
                        "outputs": { "$push": {
                            "$cond": [
                                { "$eq": [ "$nft_outputs.output_id.transaction_id", "$nft_outputs._id" ] },
                                {
                                    "id": "$nft_outputs.output_id",
                                    "asset_id": "$nft_outputs.asset_id"
                                },
                                null
                            ]
                        } },
                    } },
                    // Filter out the `null`s created in the previous stage.
                    // Note: not really necessary, but may reduce risk of bugs.
                    doc! { "$project": {
                        "_id": 1,
                        "inputs": {  "$filter": {
                            "input": "$inputs",
                            "as": "item",
                            "cond": { "$ne": [ "$$item", null ] }
                        } },
                        "outputs": { "$filter": {
                            "input": "$outputs",
                            "as": "item",
                            "cond": { "$ne": [ "$$item", null ] }
                        } }
                    } },
                    // Produce the relevant analytics.
                    doc! {
                        "$group": {
                            "_id": null,
                            "created_count": {
                                "$sum": { "$size": {
                                    "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": { "$eq": [ "$$item", NftId::implicit() ] }
                                    }
                                } }
                            },
                            "transferred_count": {
                                "$sum": {  "$size": { "$setIntersection": [
                                    { "$filter": {
                                        "input": "$inputs.asset_id",
                                        "as": "item",
                                        "cond": { "$ne": [ "$$item", NftId::implicit() ] }
                                    } },
                                    { "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": { "$ne": [ "$$item", NftId::implicit() ] }
                                    } }
                                ] } }
                            },
                            "destroyed_count": {
                                "$sum": {  "$size": { "$setDifference": [
                                    { "$filter": {
                                        "input": "$inputs.asset_id",
                                        "as": "item",
                                        "cond": { "$ne": [ "$$item", NftId::implicit() ] }
                                    } },
                                    { "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": { "$ne": [ "$$item", NftId::implicit() ] }
                                    } }
                                ] } }
                            }
                        }
                    },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
    }

    /// Gathers analytics about alias outputs that were created/transferred/burned in the given milestone.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_alias_output_activity_analytics(
        &self,
        index: MilestoneIndex,
    ) -> Result<AliasActivityAnalyticsResult, Error> {
        Ok(self
            .aggregate(
                vec![
                    // Match all alias outputs in the given milestone.
                    doc! { "$match": {
                        "$and": [
                            { "$or": [
                                { "metadata.booked.milestone_index": index },
                                { "metadata.spent_metadata.spent.milestone_index": index },
                            ] },
                            { "output.kind": "alias" },
                        ]
                    } },
                    // Screen outputs for being booked and/or spent. An output that was booked and
                    // spent will appear in both arrays, but with different ids.
                    doc! { "$facet": {
                        "booked_screening": [
                            { "$project": {
                                "_id": {
                                    "$cond": [
                                        { "$eq": [ "$metadata.booked.milestone_index", index ] },
                                        "$_id.transaction_id",
                                        "$metadata.spent_metadata.transaction_id"
                                    ]
                                },
                                "output_id": "$_id",
                                "asset_id": "$output.alias_id",
                                "state_index": "$output.state_index",
                                "governor_address": "$output.governor_address_unlock_condition.address",
                            } }
                        ],
                        "spent_screening": [
                            {"$project": {
                                "_id": {
                                    "$cond": [
                                        { "$eq": [ "$metadata.spent_metadata.spent.milestone_index", index ] },
                                        "$metadata.spent_metadata.transaction_id",
                                        "$_id.transaction_id"
                                    ]
                                },
                                "output_id": "$_id",
                                "asset_id": "$output.alias_id",
                                "state_index": "$output.state_index",
                                "governor_address": "$output.governor_address_unlock_condition.address",
                                }
                            }
                        ]
                    } },
                    // Merge both arrays from the previous facet operation. That will remove duplicates (outputs
                    // where each screening produced the same result) and keep outputs that were booked and spent
                    // within the same milestone. 
                    doc! { "$project": { "alias_outputs": { "$setUnion": [ "$booked_screening", "$spent_screening" ] } } },
                    doc! { "$unwind": { "path": "$alias_outputs" } },
                    // Reconstruct the inputs and outputs for the given asset.
                    doc! { "$group": {
                        "_id": { 
                            "tx_id": "$alias_outputs._id",
                            "asset_id": "$alias_outputs.asset_id",
                        },
                        "inputs": { "$push": {
                            "$cond": [
                                { "$ne": [ "$alias_outputs.output_id.transaction_id", "$alias_outputs._id" ] },
                                {
                                    "id": "$alias_outputs.output_id",
                                    "asset_id": "$alias_outputs.asset_id",
                                    "state_index": "$alias_outputs.state_index",
                                    "governor_address": "$alias_outputs.governor_address",
                                },
                                null
                            ]
                        } },
                        "outputs": { "$push": {
                            "$cond": [
                                { "$eq": [ "$alias_outputs.output_id.transaction_id", "$alias_outputs._id" ] },
                                {
                                    "id": "$alias_outputs.output_id",
                                    "asset_id": "$alias_outputs.asset_id",
                                    "state_index": "$alias_outputs.state_index",
                                    "governor_address": "$alias_outputs.governor_address",
                                },
                                null
                            ]
                        } },
                    } },
                    // Filter out the `null`s created in the previous stage.
                    // Note: not really necessary, but may reduce risk of bugs.
                    doc! { "$project": {
                        "_id": 1,
                        "inputs": {  "$filter": {
                            "input": "$inputs",
                            "as": "item",
                            "cond": { "$ne": [ "$$item", null ] }
                        } },
                        "outputs": { "$filter": {
                            "input": "$outputs",
                            "as": "item",
                            "cond": { "$ne": [ "$$item", null ] }
                        } }
                    } },
                    // Add fields that indicate whether state index and/or govnernor address changed.
                    doc! { "$project": {
                        "_id": 1,
                        "inputs": 1,
                        "outputs": 1,
                        "state_changed": { "$cond": [ 
                            { "$and": [
                                { "$gt": [ {"$size": "$inputs" }, 0 ] },
                                { "$gt": [ {"$size": "$outputs" }, 0] },
                                { "$lt": [ { "$max": "$inputs.state_index" }, { "$max": "$outputs.state_index" } ] }
                            ] }, 1, 0 ] },
                        "governor_address_changed": { "$cond": [ 
                            { "$and": [
                                { "$gt": [ {"$size": "$inputs" }, 0 ] },
                                { "$gt": [ {"$size": "$outputs" }, 0] },
                                { "$ne": [ { "$first": "$inputs.governor_address" }, { "$first": "$outputs.governor_address" } ] },
                            ] }, 1, 0 ] },
                    } },
                    // Produce the relevant analytics.
                    doc! {
                        "$group": {
                            "_id": null,
                            "created_count": {
                                "$sum": { "$size": {
                                    "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": { "$eq": [ "$$item", AliasId::implicit() ] }
                                    }
                                } }
                            },
                            "state_changed_count": {
                                "$sum": "$state_changed",
                            },
                            "governor_changed_count": {
                                "$sum": "$governor_address_changed",
                            },
                            "destroyed_count": {
                                "$sum": {  "$size": { "$setDifference": [
                                    { "$filter": {
                                        "input": "$inputs.asset_id",
                                        "as": "item",
                                        "cond": { "$ne": [ "$$item", AliasId::implicit() ] }
                                    } },
                                    { "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": { "$ne": [ "$$item", AliasId::implicit() ] }
                                    } }
                                ] } }
                            }
                        }
                    },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
    }
}
