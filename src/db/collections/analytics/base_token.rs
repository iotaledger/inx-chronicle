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
                doc! { "$set": {
                    "new_booked_output": {
                      "$cond": [ { "$eq": [ "$metadata.booked.milestone_index", milestone_index ] }, true, false ]
                    },
                    "new_spent_output": {
                      "$cond": [ { "$eq": [ "$metadata.spent_metadata.spent.milestone_index", milestone_index ] }, true, false ]
                    }
                } },
                doc! { "$facet": {
                    "booked_outputs": [ 
                        { "$match": { "new_booked_output": true } },
                        { "$group": { 
                            "_id": {
                                "tx": "$_id.transaction_id",
                                "address": "$details.address"
                            },
                            "amount": { "$sum": { "$toDecimal": "$output.amount" } },
                        } }
                    ],
                    "spent_outputs": [ 
                        { "$match": { "new_spent_output": true } },
                        { "$group": { 
                            "_id": {
                                "tx": "$metadata.spent_metadata.transaction_id",
                                "address": "$details.address"
                            },
                            "amount": { "$sum": { "$toDecimal": "$output.amount" } },
                        } }
                    ],
                } },
                doc! { "$unwind": {
                    "path": "$booked_outputs",
                } },
                doc! { "$project": {
                    "booked_outputs": 1,
                    "sent_back_addr": { "$first": {
                        "$filter": {
                        "input": "$spent_outputs",
                        "as": "item",
                        "cond": { "$and": [ 
                            { "$eq": ["$$item._id.tx", "$booked_outputs._id.tx"] },
                            { "$eq": ["$$item._id.address", "$booked_outputs._id.address"] },
                        ] }
                        }
                    } }
                } },
                doc! { "$project": {
                    "booked_amount": "$booked_outputs.amount",
                    "spent_amount": { "$ifNull": ["$sent_back_addr.amount", 0] },
                } },
                doc! { "$group": {
                    "_id": null,
                    "booked_amount": { "$sum": "$booked_amount" },
                    "transferred_amount": { "$sum": { 
                        "$cond": [ { "$gt": [ "$booked_amount", "$spent_amount"] }, { "$subtract": ["$booked_amount", "$spent_amount"] }, 0   
                    ] } }
                } },
                doc! { "$project": {
                    "booked_value": { "$toString": "$booked_amount" },
                    "transferred_value": { "$toString": "$transferred_amount" },
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
