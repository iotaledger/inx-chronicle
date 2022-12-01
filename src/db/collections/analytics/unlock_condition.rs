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
    db::{collections::{BlockCollection, OutputCollection}, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the statistics about the token claiming process.
#[derive(Debug)]
pub struct UnlockConditionAnalytics;

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
struct UnlockConditionAnalyticsResult {
    timelock_count: u64,
    timelock_value: d128,
    expiration_count: u64,
    expiration_value: d128,
    storage_deposit_return_count: u64,
    storage_deposit_return_value: d128,
}

#[async_trait]
impl Analytic for UnlockConditionAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Box<dyn Measurement>, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_block_activity_analytics(milestone_index)
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
    /// Gets analytics about unlock conditions.
    #[tracing::instrument(skip(self), err, level = "trace")]
    async fn get_unlock_condition_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<UnlockConditionAnalyticsResult, Error> {
        #[derive(Default, Deserialize)]
        struct Res {
            count: u64,
            value: d128,
        }

        let query = |kind: &'static str| async move {
            Result::<Res, Error>::Ok(
                self.aggregate(
                    vec![
                        doc! { "$match": {
                            format!("output.{kind}"): { "$exists": true },
                            "metadata.booked.milestone_index": { "$lte": ledger_index },
                            "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                        } },
                        doc! { "$group": {
                            "_id": null,
                            "count": { "$sum": 1 },
                            "value": { "$sum": { "$toDecimal": "$output.amount" } },
                        } },
                        doc! { "$project": {
                            "count": 1,
                            "value": { "$toString": "$value" },
                        } },
                    ],
                    None,
                )
                .await?
                .try_next()
                .await?
                .unwrap_or_default(),
            )
        };

        let (timelock, expiration, sdruc) = tokio::try_join!(
            query("timelock_unlock_condition"),
            query("expiration_unlock_condition"),
            query("storage_deposit_return_unlock_condition"),
        )?;

        Ok(UnlockConditionAnalyticsResult {
            timelock_count: timelock.count,
            timelock_value: timelock.value,
            expiration_count: expiration.count,
            expiration_value: expiration.value,
            storage_deposit_return_count: sdruc.count,
            storage_deposit_return_value: sdruc.value,
        })
    }
}

impl Measurement for PerMilestone<UnlockConditionAnalyticsResult> {
    fn into_write_query(&self) -> influxdb::WriteQuery {
        influxdb::Timestamp::from(self.milestone_timestamp)
        .into_query("stardust_unlock_conditions")
        .add_field("milestone_index", self.milestone_index)
        .add_field("expiration_count", self.data.expiration_count)
        .add_field(
            "expiration_value",
            self.data.expiration_value.to_string().parse::<u64>().unwrap(),
        )
        .add_field("timelock_count", self.data.timelock_count)
        .add_field(
            "timelock_value",
            self.data.timelock_value.to_string().parse::<u64>().unwrap(),
        )
        .add_field("storage_deposit_return_count", self.data.storage_deposit_return_count)
        .add_field(
            "storage_deposit_return_value",
            self.data
                .storage_deposit_return_value
                .to_string()
                .parse::<u64>()
                .unwrap(),
        )
    }
}
