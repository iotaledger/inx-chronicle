// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use super::{Analytic, Measurement, PerMilestone};
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
    ) -> Option<Result<Measurement, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_output_activity_analytics(milestone_index)
            .await;
        Some(match res {
            Ok(measurement) => Ok(Measurement::OutputActivityAnalytics(PerMilestone {
                milestone_index,
                milestone_timestamp,
                inner: measurement,
            })),
            Err(err) => Err(err),
        })
    }
}

impl OutputCollection {
    /// TODO: Merge with above
    /// Gathers analytics about outputs that were created/transferred/burned in the given milestone.
    #[tracing::instrument(skip(self), err, level = "trace")]
    async fn get_output_activity_analytics(
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
