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
        let nft_activity = db
            .collection::<OutputCollection>()
            .get_nft_output_activity_analytics(milestone_index)
            .await;

        let alias_activity = db
            .collection::<OutputCollection>()
            .get_alias_output_activity_analytics(milestone_index)
            .await;

        Ok(Some(Measurement::OutputActivityAnalytics(PerMilestone {
            milestone_index,
            milestone_timestamp,
            inner: OutputActivityAnalyticsResult {
                nft: nft_activity?,
                alias: alias_activity?,
            },
        })))
    }
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
                    // Match outputs from transactions in this milestone.
                    doc! { "$match": {
                        "$or": [
                            { "metadata.booked.milestone_index": index },
                            { "metadata.spent_metadata.spent.milestone_index": index },
                    ] } },
                    // Move all booked and all spent outputs into separate arrays.
                    // They all get assigned the applied transaction id they appear in.
                    // Booked *and* spent outputs land in both.
                    doc! { "$facet": {
                        "booked": [
                            { "$match": { "output.kind": "nft" } },
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
                        "spent": [
                            { "$match": { "output.kind": "nft" } },
                            {"$project": {
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
                    } },
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
                                "$sum": { "$size": {
                                    "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": {  "$ne": [ "$$item", NftId::implicit() ] }
                                    }
                                } }
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
                    // Match outputs from transactions in this milestone.
                    doc! { "$match": {
                        "$or": [
                            { "metadata.booked.milestone_index": index },
                            { "metadata.spent_metadata.spent.milestone_index": index },
                    ] } },
                    // Move all booked and all spent outputs into separate arrays.
                    // They all get assigned the applied transaction id they appear in.
                    // Booked *and* spent outputs land in both.
                    doc! { "$facet": {
                        "booked": [
                            { "$match": { "output.kind": "alias" } },
                            { "$project": {
                                "_id": {
                                    "$cond": [
                                        { "$eq": [ "$metadata.booked.milestone_index", index ] },
                                        "$_id.transaction_id",
                                        "$metadata.spent_metadata.transaction_id"
                                    ]
                                },
                                "output_id": "$_id",
                                "asset_id": "alias",
                            } }
                        ],
                        "spent": [
                            { "$match": { "output.kind": "alias" } },
                            {"$project": {
                                "_id": {
                                    "$cond": [
                                        { "$eq": [ "$metadata.spent_metadata.spent.milestone_index", index ] },
                                        "$metadata.spent_metadata.transaction_id",
                                        "$_id.transaction_id"
                                    ]
                                },
                                "output_id": "$_id",
                                "asset_id": "alias",
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
                    } },
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
                            "created_count": {
                                "$sum": { "$size": {
                                    "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": { "$eq": [ "$$item", AliasId::implicit() ] }
                                    }
                                } }
                            },
                            "governor_changed_count": {

                            },
                            "state_changed_count":  {

                            },
                            "transferred_count": {
                                "$sum": { "$size": {
                                    "$filter": {
                                        "input": "$outputs.asset_id",
                                        "as": "item",
                                        "cond": {  "$ne": [ "$$item", AliasId::implicit()  ] }
                                    }
                                } }
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
