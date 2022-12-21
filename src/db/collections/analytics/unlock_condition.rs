// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use decimal::d128;
use derive_more::{AddAssign, SubAssign};
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use super::{Analytic, Error, Measurement, PerMilestone};
use crate::{
    db::{collections::OutputCollection, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the statistics about the token claiming process.
#[derive(Debug, Default)]
pub struct UnlockConditionAnalytics {
    prev: Option<(MilestoneIndex, UnlockConditionAnalyticsResult)>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize, AddAssign, SubAssign)]
#[allow(missing_docs)]
pub struct UnlockConditionAnalyticsResult {
    pub timelock_count: u64,
    pub timelock_value: d128,
    pub expiration_count: u64,
    pub expiration_value: d128,
    pub storage_deposit_return_count: u64,
    pub storage_deposit_return_value: d128,
}

#[derive(Default, Deserialize)]
struct Sums {
    count: u64,
    value: d128,
}

#[async_trait]
impl Analytic for UnlockConditionAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        let res = if let Some(prev) = self.prev.as_mut() {
            debug_assert!(
                milestone_index == prev.0 + 1,
                "Expected {milestone_index} found {}",
                prev.0 + 1
            );
            db.collection::<OutputCollection>()
                .update_unlock_condition_analytics(&mut prev.1, milestone_index)
                .await?;
            *prev.0 = milestone_index.into();
            prev.1
        } else {
            self.prev
                .insert((
                    milestone_index,
                    db.collection::<OutputCollection>()
                        .get_unlock_condition_analytics(milestone_index)
                        .await?,
                ))
                .1
        };

        Ok(Some(Measurement::UnlockConditionAnalytics(PerMilestone {
            milestone_index,
            milestone_timestamp,
            inner: res,
        })))
    }
}

impl OutputCollection {
    /// Gets analytics about unlock conditions.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_unlock_condition_analytics(
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

    /// Gets analytics about unlock conditions.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn update_unlock_condition_analytics(
        &self,
        prev_analytics: &mut UnlockConditionAnalyticsResult,
        ledger_index: MilestoneIndex,
    ) -> Result<(), Error> {
        let query = |kind: &'static str| async move {
            tokio::try_join!(
                async {
                    Result::<Sums, Error>::Ok(
                        self.aggregate(
                            vec![
                                doc! { "$match": {
                                    format!("output.{kind}"): { "$exists": true },
                                    "metadata.booked.milestone_index": ledger_index,
                                    "metadata.spent_metadata.spent.milestone_index": { "$ne": ledger_index }
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
                },
                async {
                    Result::<Sums, Error>::Ok(
                        self.aggregate(
                            vec![
                                doc! { "$match": {
                                    format!("output.{kind}"): { "$exists": true },
                                    "metadata.booked.milestone_index": { "$ne": ledger_index },
                                    "metadata.spent_metadata.spent.milestone_index": ledger_index
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
                }
            )
        };

        let (
            (timelock_created, timelock_consumed),
            (expiration_created, expiration_consumed),
            (sdruc_created, sdruc_consumed),
        ) = tokio::try_join!(
            query("timelock_unlock_condition"),
            query("expiration_unlock_condition"),
            query("storage_deposit_return_unlock_condition"),
        )?;

        let created = UnlockConditionAnalyticsResult {
            timelock_count: timelock_created.count,
            timelock_value: timelock_created.value,
            expiration_count: expiration_created.count,
            expiration_value: expiration_created.value,
            storage_deposit_return_count: sdruc_created.count,
            storage_deposit_return_value: sdruc_created.value,
        };
        let consumed = UnlockConditionAnalyticsResult {
            timelock_count: timelock_consumed.count,
            timelock_value: timelock_consumed.value,
            expiration_count: expiration_consumed.count,
            expiration_value: expiration_consumed.value,
            storage_deposit_return_count: sdruc_consumed.count,
            storage_deposit_return_value: sdruc_consumed.value,
        };
        *prev_analytics += created;
        *prev_analytics -= consumed;

        Ok(())
    }
}
