// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use influxdb::{InfluxDbWriteable, Timestamp};

use super::*;
use crate::{
    db::influxdb::{InfluxDb, InfluxDbMeasurement},
    types::stardust::milestone::MilestoneTimestamp,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Schema<A> {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub analytics: A,
}

impl InfluxDb {
    async fn insert_analytics<A>(
        &self,
        milestone_timestamp: MilestoneTimestamp,
        milestone_index: MilestoneIndex,
        analytics: A,
    ) -> Result<(), influxdb::Error>
    where
        Schema<A>: InfluxDbMeasurement,
    {
        self.insert(Schema {
            milestone_timestamp,
            milestone_index,
            analytics,
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
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.native_tokens),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.nfts),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.storage_deposits),
            self.insert_analytics(milestone_timestamp, milestone_index, analytics.claimed_tokens),
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

impl InfluxDbWriteable for Schema<AddressActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("total_count", self.analytics.total_count)
            .add_field("receiving_count", self.analytics.receiving_count)
            .add_field("sending_count", self.analytics.sending_count)
    }
}

impl InfluxDbMeasurement for Schema<AddressActivityAnalytics> {
    const NAME: &'static str = "stardust_address_activity";
}

impl InfluxDbWriteable for Schema<AddressAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("address_with_balance_count", self.analytics.address_with_balance_count)
    }
}

impl InfluxDbMeasurement for Schema<AddressAnalytics> {
    const NAME: &'static str = "stardust_addresses";
}

impl InfluxDbWriteable for Schema<LedgerOutputAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("basic_count", self.analytics.basic_count)
            .add_field(
                "basic_value",
                self.analytics.basic_value.to_string().parse::<u64>().unwrap(),
            )
            .add_field("alias_count", self.analytics.alias_count)
            .add_field(
                "alias_value",
                self.analytics.alias_value.to_string().parse::<u64>().unwrap(),
            )
            .add_field("foundry_count", self.analytics.foundry_count)
            .add_field(
                "foundry_value",
                self.analytics.foundry_value.to_string().parse::<u64>().unwrap(),
            )
            .add_field("nft_count", self.analytics.nft_count)
            .add_field(
                "nft_value",
                self.analytics.nft_value.to_string().parse::<u64>().unwrap(),
            )
            .add_field("treasury_count", self.analytics.treasury_count)
            .add_field(
                "treasury_value",
                self.analytics.treasury_value.to_string().parse::<u64>().unwrap(),
            )
    }
}

impl InfluxDbMeasurement for Schema<LedgerOutputAnalytics> {
    const NAME: &'static str = "stardust_ledger_outputs";
}

impl InfluxDbWriteable for Schema<BaseTokenActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field(
                "transferred_value",
                self.analytics.transferred_value.to_string().parse::<u64>().unwrap(),
            )
    }
}

impl InfluxDbMeasurement for Schema<BaseTokenActivityAnalytics> {
    const NAME: &'static str = "stardust_base_token_activity";
}

impl InfluxDbWriteable for Schema<LedgerSizeAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field(
                "total_storage_deposit_value",
                self.analytics
                    .total_storage_deposit_value
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

impl InfluxDbMeasurement for Schema<LedgerSizeAnalytics> {
    const NAME: &'static str = "stardust_ledger_size";
}

impl InfluxDbWriteable for Schema<ClaimedTokensAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("claimed_count", self.analytics.claimed_count)
            .add_field(
                "claimed_value",
                self.analytics.claimed_value.to_string().parse::<u64>().unwrap(),
            )
    }
}

impl InfluxDbMeasurement for Schema<ClaimedTokensAnalytics> {
    const NAME: &'static str = "stardust_claiming_rewards";
}

impl InfluxDbWriteable for Schema<PayloadActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("transaction_count", self.analytics.transaction_count)
            .add_field("treasury_transaction_count", self.analytics.treasury_transaction_count)
            .add_field("milestone_count", self.analytics.milestone_count)
            .add_field("tagged_data_count", self.analytics.tagged_data_count)
            .add_field("no_payload_count", self.analytics.no_payload_count)
    }
}

impl InfluxDbMeasurement for Schema<PayloadActivityAnalytics> {
    const NAME: &'static str = "stardust_payload_activity";
}

impl InfluxDbWriteable for Schema<TransactionActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("confirmed_count", self.analytics.confirmed_count)
            .add_field("conflicting_count", self.analytics.conflicting_count)
            .add_field("no_transaction_count", self.analytics.no_transaction_count)
    }
}

impl InfluxDbMeasurement for Schema<TransactionActivityAnalytics> {
    const NAME: &'static str = "stardust_transaction_activity";
}

impl InfluxDbWriteable for Schema<ProtocolParameters> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("token_supply", self.analytics.token_supply)
            .add_field("min_pow_score", self.analytics.min_pow_score)
            .add_field("below_max_depth", self.analytics.below_max_depth)
            .add_field("v_byte_cost", self.analytics.rent_structure.v_byte_cost)
            .add_field("v_byte_factor_key", self.analytics.rent_structure.v_byte_factor_key)
            .add_field("v_byte_factor_data", self.analytics.rent_structure.v_byte_factor_data)
    }
}

impl InfluxDbMeasurement for Schema<ProtocolParameters> {
    const NAME: &'static str = "stardust_protocol_params";
}

impl InfluxDbWriteable for Schema<AliasActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("created_count", self.analytics.created_count)
            .add_field("state_changed_count", self.analytics.state_changed_count)
            .add_field("governor_changed_count", self.analytics.governor_changed_count)
            .add_field("destroyed_count", self.analytics.destroyed_count)
    }
}

impl InfluxDbMeasurement for Schema<AliasActivityAnalytics> {
    const NAME: &'static str = "stardust_alias_activity";
}

impl InfluxDbWriteable for Schema<FoundryActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("created_count", self.analytics.created_count)
            .add_field("transferred_count", self.analytics.transferred_count)
            .add_field("destroyed_count", self.analytics.destroyed_count)
    }
}

impl InfluxDbMeasurement for Schema<FoundryActivityAnalytics> {
    const NAME: &'static str = "stardust_foundry_activity";
}

impl InfluxDbWriteable for Schema<NftActivityAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("created_count", self.analytics.created_count)
            .add_field("transferred_count", self.analytics.transferred_count)
            .add_field("destroyed_count", self.analytics.destroyed_count)
    }
}

impl InfluxDbMeasurement for Schema<NftActivityAnalytics> {
    const NAME: &'static str = "stardust_nft_activity";
}

impl InfluxDbWriteable for Schema<UnlockConditionAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("expiration_count", self.analytics.expiration_count)
            .add_field(
                "expiration_value",
                self.analytics.expiration_value.to_string().parse::<u64>().unwrap(),
            )
            .add_field("timelock_count", self.analytics.timelock_count)
            .add_field(
                "timelock_value",
                self.analytics.timelock_value.to_string().parse::<u64>().unwrap(),
            )
            .add_field(
                "storage_deposit_return_count",
                self.analytics.storage_deposit_return_count,
            )
            .add_field(
                "storage_deposit_return_value",
                self.analytics
                    .storage_deposit_return_value
                    .to_string()
                    .parse::<u64>()
                    .unwrap(),
            )
    }
}

impl InfluxDbMeasurement for Schema<UnlockConditionAnalytics> {
    const NAME: &'static str = "stardust_unlock_conditions";
}

impl InfluxDbWriteable for Schema<SyncAnalytics> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        Timestamp::from(self.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.milestone_index)
            .add_field("sync_time", self.analytics.sync_time)
    }
}

impl InfluxDbMeasurement for Schema<SyncAnalytics> {
    const NAME: &'static str = "stardust_sync_analytics";
}
