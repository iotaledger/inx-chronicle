// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::TryStreamExt;
use influxdb::InfluxDbWriteable;
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use super::{Analytic, Measurement, PerMilestone};
use crate::{
    db::{collections::BlockCollection, MongoDb, MongoDbCollectionExt},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

/// Computes the statistics about the token claiming process.
#[derive(Debug)]
pub struct BlockActivityAnalytics;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadActivityAnalyticsResult {
    pub transaction_count: u32,
    pub treasury_transaction_count: u32,
    pub milestone_count: u32,
    pub tagged_data_count: u32,
    pub no_payload_count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionActivityAnalyticsResult {
    pub confirmed_count: u32,
    pub conflicting_count: u32,
    pub no_transaction_count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockActivityAnalyticsResult {
    pub payload: PayloadActivityAnalyticsResult,
    pub transaction: TransactionActivityAnalyticsResult,
}

#[async_trait]
impl Analytic for BlockActivityAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Box<dyn Measurement>, Error>> {
        let res = db
            .collection::<BlockCollection>()
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

impl BlockCollection {
    /// TODO: Merge with above
    /// Gathers past-cone payload activity statistics for a given milestone.
    #[tracing::instrument(skip(self), err, level = "trace")]
    async fn get_block_activity_analytics(&self, index: MilestoneIndex) -> Result<BlockActivityAnalyticsResult, Error> {
        Ok(self
            .aggregate(
                vec![
                    doc! { "$match": { "metadata.referenced_by_milestone_index": index } },
                    doc! { "$group": {
                        "_id": null,
                        "transaction_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$block.payload.kind", "transaction" ] }, 1 , 0 ]
                        } },
                        "treasury_transaction_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$block.payload.kind", "treasury_transaction" ] }, 1 , 0 ]
                        } },
                        "milestone_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$block.payload.kind", "milestone" ] }, 1 , 0 ]
                        } },
                        "tagged_data_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$block.payload.kind", "tagged_data" ] }, 1 , 0 ]
                        } },
                        "no_payload_count": { "$sum": {
                            "$cond": [ { "$not": "$block.payload" }, 1 , 0 ]
                        } },
                        "confirmed_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$metadata.inclusion_state", "included" ] }, 1 , 0 ]
                        } },
                        "conflicting_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$metadata.inclusion_state", "conflicting" ] }, 1 , 0 ]
                        } },
                        "no_transaction_count": { "$sum": {
                            "$cond": [ { "$eq": [ "$metadata.inclusion_state", "no_transaction" ] }, 1 , 0 ]
                        } },
                    } },
                    doc! { "$project": {
                        "payload": {
                            "transaction_count": "$transaction_count",
                            "treasury_transaction_count": "$treasury_transaction_count",
                            "milestone_count": "$milestone_count",
                            "tagged_data_count": "$tagged_data_count",
                            "no_payload_count": "$no_payload_count",
                        },
                        "transaction": {
                            "confirmed_count": "$confirmed_count",
                            "conflicting_count": "$conflicting_count",
                            "no_transaction_count": "$no_transaction_count",
                        }
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

impl Measurement for PerMilestone<BlockActivityAnalyticsResult> {
    fn into_write_query(&self) -> influxdb::WriteQuery {
        influxdb::Timestamp::from(self.milestone_timestamp)
            .into_query("stardust_block_activity")
            .add_field("milestone_index", self.milestone_index)
            .add_field("transaction_count", self.measurement.payload.transaction_count)
            .add_field(
                "treasury_transaction_count",
                self.measurement.payload.treasury_transaction_count,
            )
            .add_field("milestone_count", self.measurement.payload.milestone_count)
            .add_field("tagged_data_count", self.measurement.payload.tagged_data_count)
            .add_field("no_payload_count", self.measurement.payload.no_payload_count)
            .add_field("confirmed_count", self.measurement.transaction.confirmed_count)
            .add_field("conflicting_count", self.measurement.transaction.conflicting_count)
            .add_field(
                "no_transaction_count",
                self.measurement.transaction.no_transaction_count,
            )
    }
}
