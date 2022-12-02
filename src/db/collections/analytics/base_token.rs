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
    /// TODO: Merge with above
    /// Gathers output analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    async fn get_base_token_activity_analytics(
        &self,
        milestone_index: MilestoneIndex,
    ) -> Result<BaseTokenActivityAnalyticsResult, Error> {
        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": milestone_index,
                    } },
                    doc! { "$group" : {
                        "_id": null,
                        "transferred_value": { "$sum": { "$toDecimal": "$output.amount" } },
                    } },
                    doc! { "$project": {
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
