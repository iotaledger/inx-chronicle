// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::Measurement;
use crate::db::influxdb::InfluxDb;

impl InfluxDb {
    /// Writes a [`Measurement`] to the InfluxDB database.
    pub async fn insert_measurement(&self, measurement: Measurement) -> Result<(), influxdb::Error> {
        self.analytics().query(influxdb::WriteQuery::from(measurement)).await?;
        Ok(())
    }
}

impl From<Measurement> for influxdb::WriteQuery {
    fn from(value: Measurement) -> Self {
        match value {
            Measurement::AddressAnalytics(m) => m
                .prepare_query("stardust_addresses")
                .add_field("address_with_balance_count", m.inner.address_with_balance_count),
            Measurement::BaseTokenActivityAnalytics(m) => m
                .prepare_query("stardust_base_token_activity")
                .add_field("booked_value", m.inner.booked_value.to_string().parse::<u64>().unwrap())
                .add_field(
                    "transferred_value",
                    m.inner.transferred_value.to_string().parse::<u64>().unwrap(),
                ),
            Measurement::BlockAnalytics(m) => m
                .prepare_query("stardust_block_activity")
                .add_field("transaction_count", m.inner.payload.transaction_count)
                .add_field("treasury_transaction_count", m.inner.payload.treasury_transaction_count)
                .add_field("milestone_count", m.inner.payload.milestone_count)
                .add_field("tagged_data_count", m.inner.payload.tagged_data_count)
                .add_field("no_payload_count", m.inner.payload.no_payload_count)
                .add_field("confirmed_count", m.inner.transaction.confirmed_count)
                .add_field("conflicting_count", m.inner.transaction.conflicting_count)
                .add_field("no_transaction_count", m.inner.transaction.no_transaction_count),
            Measurement::DailyActiveAddressAnalytics(t) => t
                .prepare_query("stardust_daily_active_addresses")
                .add_field("count", t.inner.count),
            Measurement::LedgerOutputAnalytics(m) => m
                .prepare_query("stardust_ledger_outputs")
                .add_field("basic_count", m.inner.basic_count)
                .add_field("basic_value", m.inner.basic_value.to_string().parse::<u64>().unwrap())
                .add_field("alias_count", m.inner.alias_count)
                .add_field("alias_value", m.inner.alias_value.to_string().parse::<u64>().unwrap())
                .add_field("foundry_count", m.inner.foundry_count)
                .add_field(
                    "foundry_value",
                    m.inner.foundry_value.to_string().parse::<u64>().unwrap(),
                )
                .add_field("nft_count", m.inner.nft_count)
                .add_field("nft_value", m.inner.nft_value.to_string().parse::<u64>().unwrap())
                .add_field("treasury_count", m.inner.treasury_count)
                .add_field(
                    "treasury_value",
                    m.inner.treasury_value.to_string().parse::<u64>().unwrap(),
                ),
            Measurement::LedgerSizeAnalytics(m) => m
                .prepare_query("stardust_ledger_size")
                .add_field(
                    "total_storage_deposit_value",
                    m.inner.total_storage_deposit_value.to_string().parse::<u64>().unwrap(),
                )
                .add_field(
                    "total_key_bytes",
                    m.inner.total_key_bytes.to_string().parse::<u64>().unwrap(),
                )
                .add_field(
                    "total_data_bytes",
                    m.inner.total_data_bytes.to_string().parse::<u64>().unwrap(),
                )
                .add_field(
                    "total_byte_cost",
                    m.inner.total_byte_cost.to_string().parse::<u64>().unwrap(),
                ),
            Measurement::OutputActivityAnalytics(m) => m
                .prepare_query("stardust_output_activity")
                .add_field("alias_created_count", m.inner.alias.created_count)
                .add_field("alias_state_changed_count", m.inner.alias.state_changed_count)
                .add_field("alias_governor_changed_count", m.inner.alias.governor_changed_count)
                .add_field("alias_destroyed_count", m.inner.alias.destroyed_count)
                .add_field("nft_created_count", m.inner.nft.created_count)
                .add_field("nft_transferred_count", m.inner.nft.transferred_count)
                .add_field("nft_destroyed_count", m.inner.nft.destroyed_count),
            Measurement::ProtocolParameters(m) => m
                .prepare_query("stardust_protocol_params")
                .add_field("token_supply", m.inner.token_supply)
                .add_field("min_pow_score", m.inner.min_pow_score)
                .add_field("below_max_depth", m.inner.below_max_depth)
                .add_field("v_byte_cost", m.inner.rent_structure.v_byte_cost)
                .add_field("v_byte_factor_key", m.inner.rent_structure.v_byte_factor_key)
                .add_field("v_byte_factor_data", m.inner.rent_structure.v_byte_factor_data),
            Measurement::UnclaimedTokenAnalytics(m) => m
                .prepare_query("stardust_unclaimed_rewards")
                .add_field("unclaimed_count", m.inner.unclaimed_count)
                .add_field(
                    "unclaimed_value",
                    m.inner.unclaimed_value.to_string().parse::<u64>().unwrap(),
                ),
            Measurement::UnlockConditionAnalytics(m) => m
                .prepare_query("stardust_unlock_conditions")
                .add_field("expiration_count", m.inner.expiration_count)
                .add_field(
                    "expiration_value",
                    m.inner.expiration_value.to_string().parse::<u64>().unwrap(),
                )
                .add_field("timelock_count", m.inner.timelock_count)
                .add_field(
                    "timelock_value",
                    m.inner.timelock_value.to_string().parse::<u64>().unwrap(),
                )
                .add_field("storage_deposit_return_count", m.inner.storage_deposit_return_count)
                .add_field(
                    "storage_deposit_return_value",
                    m.inner.storage_deposit_return_value.to_string().parse::<u64>().unwrap(),
                ),
        }
    }
}
