// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::{Analytic, Error, Measurement, PerMilestone};
use crate::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollectionExt},
    types::{
        stardust::{
            block::output::{AliasId, NftId},
            milestone::MilestoneTimestamp,
        },
        tangle::MilestoneIndex,
    },
};

/// Computes the number of addresses that hold a balance.
#[derive(Debug)]
pub struct OutputActivityAnalytics;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
#[serde(default)]
pub struct OutputActivityAnalyticsResult {
    pub alias: AliasActivityAnalyticsResult,
    pub nft: NftActivityAnalyticsResult,
}

#[async_trait]
impl Analytic for OutputActivityAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        db.collection::<OutputCollection>()
            .get_output_activity_analytics(milestone_index)
            .await
            .map(|measurement| {
                Some(Measurement::OutputActivityAnalytics(PerMilestone {
                    milestone_index,
                    milestone_timestamp,
                    inner: measurement,
                }))
            })
    }
}

impl OutputCollection {
    /// Gathers analytics about outputs that were created/transferred/burned in the given milestone.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_output_activity_analytics(
        &self,
        index: MilestoneIndex,
        output_kind: &str,
    ) -> Result<OutputActivityAnalyticsResult, Error> {
        let (asset_id, asset_implicit_id) = match output_kind {
            "nft" => (doc! { "$output.nft_id" }, NftId::implicit()),
            "alias" => (doc! { "$output.alias_id", AliasId::implicit()),
            _ => return Err(format_err!("Unknown output kind: {}", output_kind)),
        };

        Ok(self
            .aggregate(
                vec![
                    // Match outputs from transactions in this milestone.
                    doc! { "$match": doc! {
                        "$or": [
                            { "metadata.booked.milestone_index": index },
                            { "metadata.spent_metadata.spent.milestone_index": index },
                    ] } },
                    // Move all booked and all spent outputs into separate arrays.
                    // They all get assigned the applied transaction id they appear in.
                    // Booked *and* spent outputs land in both.
                    doc! { "$facet": {
                        "booked": [
                            { "$match": { "output.kind": output_kind } },
                            { "$project": {
                                "_id": {
                                    "$cond": [
                                        { "$eq": [ "$metadata.booked.milestone_index", index ] },
                                        "$_id.transaction_id",
                                        "$metadata.spent_metadata.transaction_id"
                                    ]
                                },
                                "output_id": "$_id",
                                "asset_id": asset_id,
                            } }
                        ],
                        "spent": [
                            { "$match": { "output.kind": output_kind } },
                            {"$project": { 
                                "_id": {
                                    "$cond": [
                                        { "$eq": [ "$metadata.spent_metadata.spent.milestone_index", index ] },
                                        "$metadata.spent_metadata.transaction_id",
                                        "$_id.transaction_id"
                                    ]
                                },
                                "output_id": "$_id",
                                "asset_id": asset_id,
                                }
                            }
                        ]
                    } },
                    // TODO: describe what's happening here.
                    doc! { "$project": { "transactions": { "$setUnion": [ "$booked", "$spent" ] } } },
                    doc! { "$unwind": { "path": "$transactions" } },
                    // Reconstruct the transaction for the given asset.
                    doc! { "$group": {
                            "_id": "$transactions._id",
                            "inputs": { "$push": { 
                                "$cond": [
                                    { "$ne": [ "$transactions.output_id.transaction_id", "$transactions._id" ] },
                                    {
                                        "id": "$transactions.output_id",
                                        "asset_id": "$transactions.asset_id"
                                    },
                                    null
                                ] 
                            } },
                            "outputs": { "$push": {
                                "$cond": [
                                    { "$eq": [ "$transactions.output_id.transaction_id", "$transactions._id" ] },
                                    {
                                        "id": "$transactions.output_id",
                                        "asset_id": "$transactions.asset_id"
                                    },
                                    null
                                ]
                            } },
                        }
                    },
                    // Filter out the `null`s so that each array only holds the right-kinded non-null outputs.
                    // Note: probably not necessary, but makes it less likely to cause bugs.
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
                    // Calculate the analytics.
                    doc! {
                        "$group": {
                            "_id": null,
                            // Newly created have an implicit ID.
                            "created_count": {
                                "$sum": { "$size": { 
                                    "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": { "$eq": [ "$$item", asset_implicit_id ] }
                                    }
                                } }
                            },
                            // Transferred assets have an explicit ID.
                            "transferred_explicit_count": {
                                "$sum": { "$size": {
                                    "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": {  "$ne": [ "$$item", asset_implicit_id  ] }
                                    }
                                } }
                            },
                            // 
                            "destroyed_explicit_count": {
                                "$sum": {  "$size": { "$setDifference": [
                                    { "$filter": {
                                        "input": "$inputs.asset_id",
                                        "as": "item",
                                        "cond": { "$ne": [ "$$item", asset_implicit_id ] }
                                    } },
                                    { "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": { "$ne": [ "$$item", asset_implicit_id ] }
                                    } }
                                ] } }
                            }
                        }
                    },
                    
                    
                    
                    /* doc! { "$facet": {
                        *     "nft_created": [
                        *         { "$match": {
                        *             "metadata.booked.milestone_index": index,
                        *             "output.nft_id": NftId::implicit(),
                        *         } },
                        *         { "$group": {
                        *             "_id": null,
                        *             "count": { "$sum": 1 },
                        *         } },
                        *     ],
                        *     "nft_changed": [
                        *         { "$match": {
                        *             "$and": [
                        *                 { "output.nft_id": { "$exists": true } },
                        *                 { "output.nft_id": { "$ne": NftId::implicit() } },
                        *             ]
                        *         } },
                        *         { "$group": {
                        *             "_id": "$output.nft_id",
                        *             "transferred": { "$sum": { "$cond": [ { "$eq": [
                        * "$metadata.booked.milestone_index", index ] }, 1, 0 ] } },
                        *             "unspent": { "$max": { "$cond": [ { "$eq": [ "$metadata.spent_metadata", null
                        * ] }, 1, 0 ] } },         } },
                        *         { "$group": {
                        *             "_id": null,
                        *             "transferred": { "$sum": "$transferred" },
                        *             "destroyed": { "$sum": { "$cond": [ { "$eq": [ "$unspent", 0 ] }, 1, 0 ] } },
                        *         } },
                        *     ],
                        *     "alias_created": [
                        *         { "$match": {
                        *             "metadata.booked.milestone_index": index,
                        *             "output.alias_id": AliasId::implicit(),
                        *         } },
                        *         { "$group": {
                        *             "_id": null,
                        *             "count": { "$sum": 1 },
                        *         } },
                        *     ],
                        *     "alias_changed": [
                        *         { "$match": {
                        *             "$and": [
                        *                 { "output.alias_id": { "$exists": true } },
                        *                 { "output.alias_id": { "$ne": AliasId::implicit() } },
                        *             ]
                        *         } },
                        *         // Group by state indexes to find where it changed
                        *         { "$group": {
                        *             "_id": { "alias_id": "$output.alias_id", "state_index": "$output.state_index"
                        * },             "total": { "$sum": { "$cond": [ { "$eq": [
                        * "$metadata.booked.milestone_index", index ] }, 1, 0 ] } },
                        *             "unspent": { "$max": { "$cond": [ { "$eq": [ "$metadata.spent_metadata", null
                        * ] }, 1, 0 ] } },             "prev_state": { "$max": { "$cond": [ {
                        * "$lt": [ "$metadata.booked.milestone_index", index ] }, "$output.state_index", 0 ] } },
                        *         } },
                        *         { "$group": {
                        *             "_id": "$_id.alias_id",
                        *             "total": { "$sum": "$total" },
                        *             "state": { "$sum": { "$cond": [ { "$ne": [ "$_id.state_index", "$prev_state" ]
                        * }, 1, 0 ] } },             "unspent": { "$max": "$unspent" },
                        *         } },
                        *         { "$group": {
                        *             "_id": null,
                        *             "total": { "$sum": "$total" },
                        *             "state": { "$sum": "$state" },
                        *             "destroyed": { "$sum": { "$cond": [ { "$eq": [ "$unspent", 0 ] }, 1, 0 ] } },
                        *         } },
                        *         { "$set": { "governor": { "$subtract": [ "$total", "$state" ] } } },
                        *     ],
                        * } },
                        * doc! { "$project": {
                        *     "alias": {
                        *         "created_count": { "$first": "$alias_created.count" },
                        *         "state_changed_count": { "$first": "$alias_changed.state" },
                        *         "governor_changed_count": { "$first": "$alias_changed.governor" },
                        *         "destroyed_count": { "$first": "$alias_changed.destroyed" },
                        *     },
                        *     "nft": {
                        *         "created_count": { "$first": "$nft_created.count" },
                        *         "transferred_count": { "$first": "$nft_changed.transferred" },
                        *         "destroyed_count": { "$first": "$nft_changed.destroyed" },
                        *     },
                        * } }, */
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
    }

    /// Gathers analytics about outputs that were created/transferred/burned in the given milestone.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_output_activity_analytics_old(
        &self,
        index: MilestoneIndex,
    ) -> Result<OutputActivityAnalyticsResult, Error> {
        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": {
                        "$or": [
                            { "metadata.booked.milestone_index": index },
                            { "metadata.spent_metadata.spent.milestone_index": index },
                        ],
                    } },
                    doc! { "$facet": {
                        "nft_created": [
                            { "$match": {
                                "metadata.booked.milestone_index": index,
                                "output.nft_id": NftId::implicit(),
                            } },
                            { "$group": {
                                "_id": null,
                                "count": { "$sum": 1 },
                            } },
                        ],
                        "nft_changed": [
                            { "$match": { 
                                "$and": [
                                    { "output.nft_id": { "$exists": true } },
                                    { "output.nft_id": { "$ne": NftId::implicit() } },
                                ]
                            } },
                            { "$group": {
                                "_id": "$output.nft_id",
                                "transferred": { "$sum": { "$cond": [ { "$eq": [ "$metadata.booked.milestone_index", index ] }, 1, 0 ] } },
                                "unspent": { "$max": { "$cond": [ { "$eq": [ "$metadata.spent_metadata", null ] }, 1, 0 ] } },
                            } },
                            { "$group": {
                                "_id": null,
                                "transferred": { "$sum": "$transferred" },
                                "destroyed": { "$sum": { "$cond": [ { "$eq": [ "$unspent", 0 ] }, 1, 0 ] } },
                            } },
                        ],
                        "alias_created": [
                            { "$match": {
                                "metadata.booked.milestone_index": index,
                                "output.alias_id": AliasId::implicit(),
                            } },
                            { "$group": {
                                "_id": null,
                                "count": { "$sum": 1 },
                            } },
                        ],
                        "alias_changed": [
                            { "$match": {
                                "$and": [
                                    { "output.alias_id": { "$exists": true } },
                                    { "output.alias_id": { "$ne": AliasId::implicit() } },
                                ]
                            } },
                            // Group by state indexes to find where it changed
                            { "$group": {
                                "_id": { "alias_id": "$output.alias_id", "state_index": "$output.state_index" },
                                "total": { "$sum": { "$cond": [ { "$eq": [ "$metadata.booked.milestone_index", index ] }, 1, 0 ] } },
                                "unspent": { "$max": { "$cond": [ { "$eq": [ "$metadata.spent_metadata", null ] }, 1, 0 ] } },
                                "prev_state": { "$max": { "$cond": [ { "$lt": [ "$metadata.booked.milestone_index", index ] }, "$output.state_index", 0 ] } },
                            } },
                            { "$group": {
                                "_id": "$_id.alias_id",
                                "total": { "$sum": "$total" },
                                "state": { "$sum": { "$cond": [ { "$ne": [ "$_id.state_index", "$prev_state" ] }, 1, 0 ] } },
                                "unspent": { "$max": "$unspent" },
                            } },
                            { "$group": {
                                "_id": null,
                                "total": { "$sum": "$total" },
                                "state": { "$sum": "$state" },
                                "destroyed": { "$sum": { "$cond": [ { "$eq": [ "$unspent", 0 ] }, 1, 0 ] } },
                            } },
                            { "$set": { "governor": { "$subtract": [ "$total", "$state" ] } } },
                        ],
                    } },
                    doc! { "$project": {
                        "alias": {
                            "created_count": { "$first": "$alias_created.count" },
                            "state_changed_count": { "$first": "$alias_changed.state" },
                            "governor_changed_count": { "$first": "$alias_changed.governor" },
                            "destroyed_count": { "$first": "$alias_changed.destroyed" },
                        },
                        "nft": {
                            "created_count": { "$first": "$nft_created.count" },
                            "transferred_count": { "$first": "$nft_changed.transferred" },
                            "destroyed_count": { "$first": "$nft_changed.destroyed" },
                        },
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default())
    }
}
