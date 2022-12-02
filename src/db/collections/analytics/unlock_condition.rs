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

/// Computes the statistics about the token claiming process.
#[derive(Debug)]
pub struct UnlockConditionAnalytics;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct UnlockConditionAnalyticsResult {
    pub timelock_count: u64,
    pub timelock_value: d128,
    pub expiration_count: u64,
    pub expiration_value: d128,
    pub storage_deposit_return_count: u64,
    pub storage_deposit_return_value: d128,
}

#[async_trait]
impl Analytic for UnlockConditionAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Measurement, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_unlock_condition_analytics(milestone_index)
            .await;
        Some(match res {
            Ok(measurement) => Ok(Measurement::UnlockConditionAnalytics(PerMilestone {
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
