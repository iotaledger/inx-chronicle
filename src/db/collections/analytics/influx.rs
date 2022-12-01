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
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.ledger_size),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.unclaimed_tokens),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.block_activity),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.unlock_conditions),
            async {
                if let Some(protocol_params) = analytics.protocol_params {
                    self.insert_analytics(milestone_timestamp, milestone_index, protocol_params)
                        .await?;
                }
                Ok(())
            }
        )?;
        Ok(())
    }
}

impl InfluxDbWriteable for AnalyticsSchema<LedgerSizeAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field(
                "total_storage_deposit_value",
                self.data
                    .total_storage_deposit_value
                    .to_string()
                    .parse::<u64>()
                    .unwrap(),
            )
            .add_field(
                "total_key_bytes",
                self.data.total_key_bytes.to_string().parse::<u64>().unwrap(),
            )
            .add_field(
                "total_data_bytes",
                self.data.total_data_bytes.to_string().parse::<u64>().unwrap(),
            )
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<LedgerSizeAnalytics> {
    const NAME: &'static str = "stardust_ledger_size";
}

impl InfluxDbWriteable for AnalyticsSchema<UnclaimedTokensAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("unclaimed_count", self.data.unclaimed_count)
            .add_field(
                "unclaimed_value",
                self.data.unclaimed_value.to_string().parse::<u64>().unwrap(),
            )
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<UnclaimedTokensAnalytics> {
    const NAME: &'static str = "stardust_unclaimed_rewards";
}

impl InfluxDbWriteable for AnalyticsSchema<BlockActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("transaction_count", self.data.payload.transaction_count)
            .add_field(
                "treasury_transaction_count",
                self.data.payload.treasury_transaction_count,
            )
            .add_field("milestone_count", self.data.payload.milestone_count)
            .add_field("tagged_data_count", self.data.payload.tagged_data_count)
            .add_field("no_payload_count", self.data.payload.no_payload_count)
            .add_field("confirmed_count", self.data.transaction.confirmed_count)
            .add_field("conflicting_count", self.data.transaction.conflicting_count)
            .add_field("no_transaction_count", self.data.transaction.no_transaction_count)
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<BlockActivityAnalytics> {
    const NAME: &'static str = "stardust_block_activity";
}

impl InfluxDbWriteable for AnalyticsSchema<ProtocolParameters> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("token_supply", self.data.token_supply)
            .add_field("min_pow_score", self.data.min_pow_score)
            .add_field("below_max_depth", self.data.below_max_depth)
            .add_field("v_byte_cost", self.data.rent_structure.v_byte_cost)
            .add_field("v_byte_factor_key", self.data.rent_structure.v_byte_factor_key)
            .add_field("v_byte_factor_data", self.data.rent_structure.v_byte_factor_data)
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<ProtocolParameters> {
    const NAME: &'static str = "stardust_protocol_params";
}

impl InfluxDbWriteable for AnalyticsSchema<UnlockConditionAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
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

impl InfluxDbMeasurement for AnalyticsSchema<UnlockConditionAnalytics> {
    const NAME: &'static str = "stardust_unlock_conditions";
}
