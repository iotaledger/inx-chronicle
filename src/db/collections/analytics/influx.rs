// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use influxdb::{InfluxDbWriteable, Timestamp};

// Bad bad bad
use super::*;
use super::{Analytics, Measurement};
use crate::{
    db::influxdb::{InfluxDb, InfluxDbMeasurement},
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

// TODO abstraction that runs all selected analytics concurrently

#[deprecated]
/// Defines data associated with a milestone that can be used by influx.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AnalyticsSchema<T> {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub data: T,
}

impl InfluxDb {
    async fn insert_analytics<A>(
        &self,
        milestone_timestamp: MilestoneTimestamp,
        milestone_index: MilestoneIndex,
        analytics: A,
    ) -> Result<(), influxdb::Error>
    where
        AnalyticsSchema<A>: InfluxDbMeasurement,
    {
        self.analytics()
            .insert(AnalyticsSchema {
                milestone_timestamp,
                milestone_index,
                data: analytics,
            })
            .await
    }

    /// TODO: Rename
    pub async fn insert_measurement(&self, measurement: Box<dyn Measurement>) -> Result<(), influxdb::Error> {
        self.analytics().query(measurement.into_write_query()).await?;
        Ok(())
    }

    /// Insert all gathered analytics.
    #[deprecated]
    pub async fn insert_all_analytics(
        &self,
        milestone_timestamp: MilestoneTimestamp,
        milestone_index: MilestoneIndex,
        analytics: Analytics,
    ) -> Result<(), influxdb::Error> {
        tokio::try_join!(
            // self.insert_analytics(milestone_timestamp, milestone_index, analytics.address_activity),
            // self.insert_analytics(milestone_timestamp, milestone_index, analytics.addresses),
            // self.insert_analytics(milestone_timestamp, milestone_index, analytics.base_token),
            // self.insert_analytics(milestone_timestamp, milestone_index, analytics.ledger_outputs),
            // self.insert_analytics(milestone_timestamp, milestone_index, analytics.output_activity),
            // self.insert_analytics(milestone_timestamp, milestone_index, analytics.ledger_size),
            // self.insert_analytics(milestone_timestamp, milestone_index, analytics.unclaimed_tokens),
            // self.insert_analytics(milestone_timestamp, milestone_index, analytics.block_activity),
            // self.insert_analytics(milestone_timestamp, milestone_index, analytics.unlock_conditions),
            // async {
            //     if let Some(protocol_params) = analytics.protocol_params {
            //         self.insert_analytics(milestone_timestamp, milestone_index, protocol_params)
            //             .await?;
            //     }
            //     Ok(())
            // }
        )?;
        Ok(())
    }
}
