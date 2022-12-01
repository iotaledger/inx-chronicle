// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use decimal::d128;
use futures::TryStreamExt;
use influxdb::InfluxDbWriteable;
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use super::{Analytic, Measurement, PerMilestone};
use crate::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the statistics about the token claiming process.
#[derive(Debug)]
pub struct UnclaimedTokenAnalytics;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct UnclaimedTokenAnalyticsResult {
    unclaimed_count: u64,
    unclaimed_value: d128,
}

#[async_trait]
impl Analytic for UnclaimedTokenAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Box<dyn Measurement>, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_unclaimed_token_analytics(milestone_index)
            .await;
        Some(match res {
            Ok(measurement) => Ok(Box::new(PerMilestone {
                milestone_index,
                milestone_timestamp,
                measurement,
            })),
            Err(err) => Err(err),
        })
    }
}

impl OutputCollection {
    /// TODO: Merge with above
    /// Gets the number of claimed tokens.
    #[tracing::instrument(skip(self), err, level = "trace")]
    async fn get_unclaimed_token_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<UnclaimedTokenAnalyticsResult, Error> {
        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$eq": 0 },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                    } },
                    doc! { "$group": {
                        "_id": null,
                        "unclaimed_count": { "$sum": 1 },
                        "unclaimed_value": { "$sum": { "$toDecimal": "$output.amount" } },
                    } },
                    doc! { "$project": {
                        "unclaimed_count": 1,
                        "unclaimed_value": { "$toString": "$unclaimed_value" },
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

impl Measurement for PerMilestone<UnclaimedTokenAnalyticsResult> {
    fn into_write_query(&self) -> influxdb::WriteQuery {
        influxdb::Timestamp::from(self.milestone_timestamp)
            .into_query("stardust_unclaimed_rewards")
            .add_field("milestone_index", self.milestone_index)
            .add_field("unclaimed_count", self.measurement.unclaimed_count)
            .add_field(
                "unclaimed_value",
                self.measurement.unclaimed_value.to_string().parse::<u64>().unwrap(),
            )
    }
}
