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

/// Computes the number of addresses that hold a balance.
#[derive(Debug)]
pub struct LedgerOutputAnalytics;

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct LedgerOutputAnalyticsResult {
    basic_count: u64,
    basic_value: d128,
    alias_count: u64,
    alias_value: d128,
    foundry_count: u64,
    foundry_value: d128,
    nft_count: u64,
    nft_value: d128,
    treasury_count: u64,
    treasury_value: d128,
}

#[async_trait]
impl Analytic for LedgerOutputAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Box<dyn Measurement>, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_ledger_output_analytics(milestone_index)
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

impl Measurement for PerMilestone<LedgerOutputAnalyticsResult> {
    fn into_write_query(&self) -> influxdb::WriteQuery {
        influxdb::Timestamp::from(self.milestone_timestamp)
        .into_query("stardust_ledger_outputs")
        .add_field("milestone_index", self.milestone_index)
        .add_field("basic_count", self.measurement.basic_count)
        .add_field("basic_value", self.measurement.basic_value.to_string().parse::<u64>().unwrap())
        .add_field("alias_count", self.measurement.alias_count)
        .add_field("alias_value", self.measurement.alias_value.to_string().parse::<u64>().unwrap())
        .add_field("foundry_count", self.measurement.foundry_count)
        .add_field(
            "foundry_value",
            self.measurement.foundry_value.to_string().parse::<u64>().unwrap(),
        )
        .add_field("nft_count", self.measurement.nft_count)
        .add_field("nft_value", self.measurement.nft_value.to_string().parse::<u64>().unwrap())
        .add_field("treasury_count", self.measurement.treasury_count)
        .add_field(
            "treasury_value",
            self.measurement.treasury_value.to_string().parse::<u64>().unwrap(),
        )
    }
}
