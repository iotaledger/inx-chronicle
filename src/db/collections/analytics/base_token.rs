// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use decimal::d128;
use futures::TryStreamExt;
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use super::{Analytic, Measurement, PerMilestone};
use crate::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the number of addresses that hold a balance.
#[derive(Debug)]
pub struct BaseTokenActivityAnalytics;

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BaseTokenActivityAnalyticsResult {
    pub booked_value: d128,
    pub transferred_value: d128,
}

#[async_trait]
impl Analytic for BaseTokenActivityAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Measurement, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_base_token_activity_analytics(milestone_index)
            .await;
        Some(match res {
            Ok(measurement) => Ok(Measurement::BaseTokenActivity(PerMilestone {
                milestone_index,
                milestone_timestamp,
                inner: measurement,
            })),
            Err(err) => Err(err),
        })
    }
}

impl OutputCollection {
    /// Gathers output analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_base_token_activity_analytics(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<BaseTokenActivityAnalyticsResult, Error> {
        Ok(self
        .aggregate(
            vec![
                doc! { "$match": {
                    "$or": [
                        { "metadata.booked.milestone_index": milestone_index },
                        { "metadata.spent_metadata.spent.milestone_index": milestone_index },
                    ]
                } },
                doc! { "$set": { "kind": {
                    "$cond": [
                        { "$eq": [ "$metadata.spent_metadata.spent.milestone_index", milestone_index ] },
                        "consumed_output",
                        "created_output"
                    ]
                } } },
                // Re-assemble the inputs and outputs per transaction.
                doc! { "$project": {
                    "_id": { "$cond": [ 
                            { "$eq": [ "$kind", "created_output" ] }, 
                            "$_id.transaction_id", 
                            "$metadata.spent_metadata.transaction_id" 
                    ] },
                    "address": "$details.address",
                    "amount": { "$toDecimal": "$output.amount" },
                    "kind": 1,
                } },
                // Note: we sum input amounts and subtract output amounts per transaction and per address. 
                // This way we make sure that amounts that were sent back to an input address within the 
                // same transaction get subtracted and are not falsely counted as a token transfer.
                doc! {
                    "$group": { 
                        "_id": {
                            "tx_id": "$_id",
                            "address": "$address"
                        },
                        "booked_value": { "$sum": { 
                            "$cond": [ { "$eq": ["$kind", "consumed_output"] }, "$amount", 0 ] } },
                        "transferred_value": { "$sum": {
                            "$cond": [ { "$eq": [ "$kind", "consumed_output" ] }, "$amount", { "$subtract": [ 0, "$amount" ] } ] 
                        } }
                    }
                },
                doc! {
                    "$group": {
                        "_id": null,
                        "booked_value": { "$sum": "$booked_value"},
                        "transferred_value": { "$sum": { 
                            "$cond": [ { "$gt": [ "$transferred_value", 0 ] }, "$transferred_value", 0 ]
                        } }
                    }
                },
                doc! { "$project": {
                    "booked_value": { "$toString": "$booked_value" },
                    "transferred_value": { "$toString": "$transferred_value" },
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
