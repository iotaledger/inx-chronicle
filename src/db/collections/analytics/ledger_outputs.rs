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

/// Computes the number of addresses that hold a balance.
#[derive(Debug, Default)]
pub struct LedgerOutputAnalytics {
    prev: Option<(MilestoneIndex, LedgerOutputAnalyticsResult)>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize, AddAssign, SubAssign)]
pub struct LedgerOutputAnalyticsResult {
    pub basic_count: u64,
    pub basic_value: d128,
    pub alias_count: u64,
    pub alias_value: d128,
    pub foundry_count: u64,
    pub foundry_value: d128,
    pub nft_count: u64,
    pub nft_value: d128,
    pub treasury_count: u64,
    pub treasury_value: d128,
}

#[async_trait]
impl Analytic for LedgerOutputAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Result<Option<Measurement>, Error> {
        let res = if let Some(prev) = self.prev.as_mut() {
            debug_assert!(milestone_index == prev.0 + 1, "Expected {milestone_index} found {}", prev.0 + 1);
            db.collection::<OutputCollection>()
                .update_ledger_output_analytics(&mut prev.1, milestone_index)
                .await?;
            *prev.0 = milestone_index.into();
            prev.1
        } else {
            self.prev.insert((
                milestone_index,
                db.collection::<OutputCollection>()
                    .get_ledger_output_analytics(milestone_index)
                    .await?,
            )).1
        };

        Ok(Some(Measurement::LedgerOutputAnalytics(PerMilestone {
            milestone_index,
            milestone_timestamp,
            inner: res,
        })))
    }
}

#[derive(Default, Deserialize)]
struct Sums {
    count: u64,
    value: d128,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct LedgerOutputsRes {
    basic: Sums,
    alias: Sums,
    foundry: Sums,
    nft: Sums,
    treasury: Sums,
}

impl From<LedgerOutputsRes> for LedgerOutputAnalyticsResult {
    fn from(res: LedgerOutputsRes) -> Self {
        Self {
            basic_count: res.basic.count,
            basic_value: res.basic.value,
            alias_count: res.alias.count,
            alias_value: res.alias.value,
            foundry_count: res.foundry.count,
            foundry_value: res.foundry.value,
            nft_count: res.nft.count,
            nft_value: res.nft.value,
            treasury_count: res.treasury.count,
            treasury_value: res.treasury.value,
        }
    }
}

impl OutputCollection {
    /// Gathers ledger (unspent) output analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn get_ledger_output_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<LedgerOutputAnalyticsResult, Error> {
        let res = self
            .aggregate::<LedgerOutputsRes>(
                vec![
                    doc! { "$match": {
                        "metadata.booked.milestone_index": { "$lte": ledger_index },
                        "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                    } },
                    doc! { "$group" : {
                        "_id": "$output.kind",
                        "count": { "$sum": 1 },
                        "value": { "$sum": { "$toDecimal": "$output.amount" } },
                    } },
                    doc! { "$group" : {
                        "_id": null,
                        "result": { "$addToSet": {
                            "k": "$_id",
                            "v": {
                                "count": "$count",
                                "value": { "$toString": "$value" },
                            }
                        } },
                    } },
                    doc! { "$replaceWith": {
                        "$arrayToObject": "$result"
                    } },
                ],
                None,
            )
            .await?
            .try_next()
            .await?
            .unwrap_or_default();

        Ok(LedgerOutputAnalyticsResult {
            basic_count: res.basic.count,
            basic_value: res.basic.value,
            alias_count: res.alias.count,
            alias_value: res.alias.value,
            foundry_count: res.foundry.count,
            foundry_value: res.foundry.value,
            nft_count: res.nft.count,
            nft_value: res.nft.value,
            treasury_count: res.treasury.count,
            treasury_value: res.treasury.value,
        })
    }

    /// Gathers ledger (unspent) output analytics and updates the analytics from the previous ledger index.
    ///
    /// NOTE: The `prev_analytics` must be from `ledger_index - 1` or the results are invalid.
    #[tracing::instrument(skip(self), err, level = "trace")]
    pub async fn update_ledger_output_analytics(
        &self,
        prev_analytics: &mut LedgerOutputAnalyticsResult,
        ledger_index: MilestoneIndex,
    ) -> Result<(), Error> {
        let (created, consumed) = tokio::try_join!(
            async {
                Result::<_, Error>::Ok(
                    self.aggregate::<LedgerOutputsRes>(
                        vec![
                            doc! { "$match": {
                                "metadata.booked.milestone_index": ledger_index,
                                "metadata.spent_metadata.spent.milestone_index": { "$ne": ledger_index }
                            } },
                            doc! { "$group" : {
                                "_id": "$output.kind",
                                "count": { "$sum": 1 },
                                "value": { "$sum": { "$toDecimal": "$output.amount" } },
                            } },
                            doc! { "$group" : {
                                "_id": null,
                                "result": { "$addToSet": {
                                    "k": "$_id",
                                    "v": {
                                        "count": "$count",
                                        "value": { "$toString": "$value" },
                                    }
                                } },
                            } },
                            doc! { "$replaceWith": {
                                "$arrayToObject": "$result"
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
                Ok(self
                    .aggregate::<LedgerOutputsRes>(
                        vec![
                            doc! { "$match": {
                                "metadata.booked.milestone_index": { "$ne": ledger_index },
                                "metadata.spent_metadata.spent.milestone_index": ledger_index
                            } },
                            doc! { "$group" : {
                                "_id": "$output.kind",
                                "count": { "$sum": 1 },
                                "value": { "$sum": { "$toDecimal": "$output.amount" } },
                            } },
                            doc! { "$group" : {
                                "_id": null,
                                "result": { "$addToSet": {
                                    "k": "$_id",
                                    "v": {
                                        "count": "$count",
                                        "value": { "$toString": "$value" },
                                    }
                                } },
                            } },
                            doc! { "$replaceWith": {
                                "$arrayToObject": "$result"
                            } },
                        ],
                        None,
                    )
                    .await?
                    .try_next()
                    .await?
                    .unwrap_or_default())
            }
        )?;
        *prev_analytics += created.into();
        *prev_analytics -= consumed.into();

        Ok(())
    }
}
