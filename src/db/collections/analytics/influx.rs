// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use influxdb::{InfluxDbWriteable, Timestamp};

use super::*;
use crate::{
    db::influxdb::{InfluxDb, InfluxDbMeasurement},
    types::stardust::milestone::MilestoneTimestamp,
};

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
        self.insert(AnalyticsSchema {
            milestone_timestamp,
            milestone_index,
            data: analytics,
        })
        .await
    }

    /// Insert all gathered analytics.
    pub async fn insert_all_analytics(
        &self,
        milestone_timestamp: MilestoneTimestamp,
        milestone_index: MilestoneIndex,
        analytics: Analytics,
    ) -> Result<(), influxdb::Error> {
        tokio::try_join!(
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.address_activity),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.addresses),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.base_token),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.ledger_outputs),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.aliases),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.nfts),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.storage_deposits),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.unclaimed_tokens),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.payload_activity),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.unlock_conditions),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.transaction_activity),
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

impl InfluxDbWriteable for AnalyticsSchema<AddressActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("total_count", self.data.total_count)
            .add_field("receiving_count", self.data.receiving_count)
            .add_field("sending_count", self.data.sending_count)
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<AddressActivityAnalytics> {
    const NAME: &'static str = "stardust_address_activity";
}

impl InfluxDbWriteable for AnalyticsSchema<AddressAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("address_with_balance_count", self.data.address_with_balance_count)
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<AddressAnalytics> {
    const NAME: &'static str = "stardust_addresses";
}

impl InfluxDbWriteable for AnalyticsSchema<LedgerOutputAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("basic_count", self.data.basic_count)
            .add_field("basic_value", self.data.basic_value.to_string().parse::<u64>().unwrap())
            .add_field("alias_count", self.data.alias_count)
            .add_field("alias_value", self.data.alias_value.to_string().parse::<u64>().unwrap())
            .add_field("foundry_count", self.data.foundry_count)
            .add_field(
                "foundry_value",
                self.data.foundry_value.to_string().parse::<u64>().unwrap(),
            )
            .add_field("nft_count", self.data.nft_count)
            .add_field("nft_value", self.data.nft_value.to_string().parse::<u64>().unwrap())
            .add_field("treasury_count", self.data.treasury_count)
            .add_field(
                "treasury_value",
                self.data.treasury_value.to_string().parse::<u64>().unwrap(),
            )
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<LedgerOutputAnalytics> {
    const NAME: &'static str = "stardust_ledger_outputs";
}

impl InfluxDbWriteable for AnalyticsSchema<BaseTokenActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field(
                "transferred_value",
                self.data.transferred_value.to_string().parse::<u64>().unwrap(),
            )
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<BaseTokenActivityAnalytics> {
    const NAME: &'static str = "stardust_base_token_activity";
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

impl InfluxDbWriteable for AnalyticsSchema<PayloadActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("transaction_count", self.data.transaction_count)
            .add_field("treasury_transaction_count", self.data.treasury_transaction_count)
            .add_field("milestone_count", self.data.milestone_count)
            .add_field("tagged_data_count", self.data.tagged_data_count)
            .add_field("no_payload_count", self.data.no_payload_count)
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<PayloadActivityAnalytics> {
    const NAME: &'static str = "stardust_payload_activity";
}

impl InfluxDbWriteable for AnalyticsSchema<TransactionActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("confirmed_count", self.data.confirmed_count)
            .add_field("conflicting_count", self.data.conflicting_count)
            .add_field("no_transaction_count", self.data.no_transaction_count)
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<TransactionActivityAnalytics> {
    const NAME: &'static str = "stardust_transaction_activity";
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

impl InfluxDbWriteable for AnalyticsSchema<AliasActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("created_count", self.data.created_count)
            .add_field("state_changed_count", self.data.state_changed_count)
            .add_field("governor_changed_count", self.data.governor_changed_count)
            .add_field("destroyed_count", self.data.destroyed_count)
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<AliasActivityAnalytics> {
    const NAME: &'static str = "stardust_alias_activity";
}

impl InfluxDbWriteable for AnalyticsSchema<FoundryActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("created_count", self.data.created_count)
            .add_field("transferred_count", self.data.transferred_count)
            .add_field("destroyed_count", self.data.destroyed_count)
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<FoundryActivityAnalytics> {
    const NAME: &'static str = "stardust_foundry_activity";
}

impl InfluxDbWriteable for AnalyticsSchema<NftActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("created_count", self.data.created_count)
            .add_field("transferred_count", self.data.transferred_count)
            .add_field("destroyed_count", self.data.destroyed_count)
    }
}

impl InfluxDbMeasurement for AnalyticsSchema<NftActivityAnalytics> {
    const NAME: &'static str = "stardust_nft_activity";
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
