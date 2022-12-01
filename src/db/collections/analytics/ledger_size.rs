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
    types::{
        stardust::{
            block::output::{AliasId, NftId},
            milestone::MilestoneTimestamp,
        },
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

/// Computes the size of the ledger.
#[derive(Debug)]
pub struct LedgerSizeAnalytics;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct LedgerSizeAnalyticsResult {
    total_storage_deposit_value: d128,
    total_key_bytes: d128,
    total_data_bytes: d128,
}

impl LedgerSizeAnalyticsResult {
    pub fn total_byte_cost(&self, protocol_params: &ProtocolParameters) -> d128 {
        let rent_structure = protocol_params.rent_structure;
        d128::from(rent_structure.v_byte_cost)
            * ((self.total_key_bytes * d128::from(rent_structure.v_byte_factor_key as u32))
                + (self.total_data_bytes * d128::from(rent_structure.v_byte_factor_data as u32)))
    }
}

#[async_trait]
impl Analytic for LedgerSizeAnalytics {
    async fn get_measurement(
        &mut self,
        db: &MongoDb,
        milestone_index: MilestoneIndex,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Option<Result<Box<dyn Measurement>, Error>> {
        let res = db
            .collection::<OutputCollection>()
            .get_ledger_size_analytics(milestone_index)
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
    /// Gathers byte cost and storage deposit analytics.
    #[tracing::instrument(skip(self), err, level = "trace")]
    async fn get_ledger_size_analytics(
        &self,
        ledger_index: MilestoneIndex,
    ) -> Result<LedgerSizeAnalyticsResult, Error> {
        Ok(self
        .aggregate(
            vec![
                doc! { "$match": {
                    "metadata.booked.milestone_index": { "$lte": ledger_index },
                    "metadata.spent_metadata.spent.milestone_index": { "$not": { "$lte": ledger_index } }
                } },
                doc! { "$group" : {
                    "_id": null,
                    "total_key_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_key_bytes" } },
                    "total_data_bytes": { "$sum": { "$toDecimal": "$details.rent_structure.num_data_bytes" } },
                    "total_storage_deposit_value": { "$sum": { "$toDecimal": "$output.storage_deposit_return_unlock_condition.amount" } },
                } },
                doc! { "$project": {
                    "total_storage_deposit_value": { "$toString": "$total_storage_deposit_value" },
                    "total_key_bytes": { "$toString": "$total_key_bytes" },
                    "total_data_bytes": { "$toString": "$total_data_bytes" },
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

impl Measurement for PerMilestone<LedgerSizeAnalyticsResult> {
    fn into_write_query(&self) -> influxdb::WriteQuery {
        influxdb::Timestamp::from(self.milestone_timestamp)
            .into_query("stardust_ledger_size")
            .add_field("milestone_index", self.milestone_index)
            .add_field(
                "total_storage_deposit_value",
                self.measurement
                    .total_storage_deposit_value
                    .to_string()
                    .parse::<u64>()
                    .unwrap(),
            )
            .add_field(
                "total_key_bytes",
                self.measurement.total_key_bytes.to_string().parse::<u64>().unwrap(),
            )
            .add_field(
                "total_data_bytes",
                self.measurement.total_data_bytes.to_string().parse::<u64>().unwrap(),
            )
    }
}
