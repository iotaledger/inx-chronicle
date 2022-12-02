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
pub struct LedgerOutputAnalytics;

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
    ) -> Option<Result<Measurement, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_ledger_output_analytics(milestone_index)
            .await;
        Some(match res {
            Ok(measurement) => Ok(Measurement::LedgerOutputAnalytics(PerMilestone {
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
    /// Gathers ledger (unspent) output analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    async fn get_ledger_output_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<LedgerOutputAnalyticsResult, Error> {
        #[derive(Default, Deserialize)]
        struct Sums {
            count: u64,
            value: d128,
        }

        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Res {
            basic: Sums,
            alias: Sums,
            foundry: Sums,
            nft: Sums,
            treasury: Sums,
        }

        let res = self
            .aggregate::<Res>(
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
}
