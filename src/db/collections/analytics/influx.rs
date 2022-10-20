// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use influxdb::{InfluxDbWriteable, Timestamp};

use super::*;
use crate::db::influxdb::{InfluxDb, InfluxDbMeasurement};

impl InfluxDb {
    /// Insert all gathered analytics.
    pub async fn insert_all_analytics(
        &self,
        milestone_timestamp: MilestoneTimestamp,
        milestone_index: MilestoneIndex,
        mut analytics: Analytics,
    ) -> Result<(), influxdb::Error> {
        self.insert(AddressAnalyticsSchema {
            milestone_timestamp,
            milestone_index,
            analytics: analytics.addresses,
        })
        .await?;
        for (kind, outputs) in analytics.outputs.drain() {
            self.insert(OutputAnalyticsSchema {
                milestone_timestamp,
                milestone_index,
                kind,
                analytics: outputs,
            })
            .await?;
        }
        for (kind, outputs) in analytics.unspent_outputs.drain() {
            self.insert(OutputAnalyticsSchema {
                milestone_timestamp,
                milestone_index,
                kind,
                analytics: outputs,
            })
            .await?;
        }
        self.insert(StorageDepositAnalyticsSchema {
            milestone_timestamp,
            milestone_index,
            analytics: analytics.storage_deposits,
        })
        .await?;
        self.insert(ClaimedTokensAnalyticsSchema {
            milestone_timestamp,
            milestone_index,
            analytics: analytics.claimed_tokens,
        })
        .await?;
        self.insert(MilestoneActivityAnalyticsSchema {
            milestone_timestamp,
            milestone_index,
            analytics: analytics.milestone_activity,
        })
        .await?;
        Ok(())
    }
}

impl InfluxDbWriteable for AddressAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_field("total_active_addresses", self.analytics.total_active_addresses)
            .add_field("receiving_addresses", self.analytics.receiving_addresses)
            .add_field("sending_addresses", self.analytics.sending_addresses)
    }
}

impl InfluxDbMeasurement for AddressAnalyticsSchema {
    const NAME: &'static str = "stardust_addresses";
}

impl InfluxDbWriteable for OutputAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_tag("kind", self.kind)
            .add_field("count", self.analytics.count)
            .add_field(
                "total_value",
                self.analytics.total_value.to_string().parse::<u64>().unwrap(),
            )
    }
}

impl InfluxDbMeasurement for OutputAnalyticsSchema {
    const NAME: &'static str = "stardust_outputs";
}

impl InfluxDbWriteable for StorageDepositAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_field(
                "storage_deposit_return_count",
                self.analytics.storage_deposit_return_count,
            )
            .add_field(
                "storage_deposit_return_total_value",
                self.analytics
                    .storage_deposit_return_total_value
                    .to_string()
                    .parse::<u64>()
                    .unwrap(),
            )
            .add_field(
                "total_key_bytes",
                self.analytics.total_key_bytes.to_string().parse::<u64>().unwrap(),
            )
            .add_field(
                "total_data_bytes",
                self.analytics.total_data_bytes.to_string().parse::<u64>().unwrap(),
            )
    }
}

impl InfluxDbMeasurement for StorageDepositAnalyticsSchema {
    const NAME: &'static str = "stardust_storage_deposits";
}

impl InfluxDbWriteable for ClaimedTokensAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_field("count", self.analytics.count.to_string().parse::<u64>().unwrap())
    }
}

impl InfluxDbMeasurement for ClaimedTokensAnalyticsSchema {
    const NAME: &'static str = "stardust_claimed_tokens";
}

impl InfluxDbWriteable for MilestoneActivityAnalyticsSchema {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_tag("milestone_index", self.milestone_index)
            .add_field("count", self.analytics.count)
            .add_field("transaction_count", self.analytics.transaction_count)
            .add_field("treasury_transaction_count", self.analytics.treasury_transaction_count)
            .add_field("milestone_count", self.analytics.milestone_count)
            .add_field("tagged_data_count", self.analytics.tagged_data_count)
            .add_field("no_payload_count", self.analytics.no_payload_count)
            .add_field("confirmed_count", self.analytics.confirmed_count)
            .add_field("conflicting_count", self.analytics.conflicting_count)
            .add_field("no_transaction_count", self.analytics.no_transaction_count)
    }
}

impl InfluxDbMeasurement for MilestoneActivityAnalyticsSchema {
    const NAME: &'static str = "stardust_milestone_activity";
}
