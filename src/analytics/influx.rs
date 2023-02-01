// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Influx Measurement implementations

use influxdb::{InfluxDbWriteable, WriteQuery};
use time::{Duration, OffsetDateTime};

use super::{
    ledger::{
        AddressActivityMeasurement, AddressBalanceMeasurement, BaseTokenActivityMeasurement, LedgerOutputMeasurement,
        LedgerSizeMeasurement, OutputActivityMeasurement, UnclaimedTokenMeasurement, UnlockConditionMeasurement,
    },
    tangle::{BlockActivityMeasurement, MilestoneSizeMeasurement},
};
use crate::{
    db::influxdb::InfluxDb,
    types::{ledger::MilestoneIndexTimestamp, stardust::milestone::MilestoneTimestamp, tangle::ProtocolParameters},
};

#[allow(missing_docs)]
pub enum Measurement {
    AddressBalance(PerMilestone<AddressBalanceMeasurement>),
    BaseTokenActivity(PerMilestone<BaseTokenActivityMeasurement>),
    BlockActivity(PerMilestone<BlockActivityMeasurement>),
    DailyActiveAddresses(TimeInterval<AddressActivityMeasurement>),
    LedgerOutputs(PerMilestone<LedgerOutputMeasurement>),
    LedgerSize(PerMilestone<LedgerSizeMeasurement>),
    MilestoneSize(PerMilestone<MilestoneSizeMeasurement>),
    OutputActivity(PerMilestone<OutputActivityMeasurement>),
    ProtocolParameters(PerMilestone<ProtocolParameters>),
    UnclaimedTokens(PerMilestone<UnclaimedTokenMeasurement>),
    UnlockConditions(PerMilestone<UnlockConditionMeasurement>),
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct PerMilestone<M> {
    pub at: MilestoneIndexTimestamp,
    pub inner: M,
}

impl<M> PerMilestone<M> {
    fn prepare_query(&self, name: impl Into<String>) -> WriteQuery {
        influxdb::Timestamp::from(self.at.milestone_timestamp)
            .into_query(name)
            .add_field("milestone_index", self.at.milestone_index)
    }
}

/// Note: We will need this later, for example for daily active addresses.
#[allow(unused)]
#[allow(missing_docs)]
pub struct TimeInterval<M> {
    pub from: OffsetDateTime,
    pub to_exclusive: OffsetDateTime,
    pub inner: M,
}

impl<M> TimeInterval<M> {
    fn prepare_query(&self, name: impl Into<String>) -> WriteQuery {
        // We subtract 1 nanosecond to get the inclusive end of the time interval.
        let timestamp = self.to_exclusive - Duration::nanoseconds(1);
        influxdb::Timestamp::from(MilestoneTimestamp::from(timestamp)).into_query(name)
    }
}

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
            Measurement::AddressBalance(m) => m
                .prepare_query("stardust_addresses")
                .add_field("address_with_balance_count", m.inner.address_with_balance_count as u64),
            Measurement::BaseTokenActivity(m) => m
                .prepare_query("stardust_base_token_activity")
                .add_field("booked_value", m.inner.booked_value)
                .add_field("transferred_value", m.inner.transferred_value),
            Measurement::BlockActivity(m) => m
                .prepare_query("stardust_block_activity")
                .add_field("transaction_count", m.inner.transaction_count as u64)
                .add_field("treasury_transaction_count", m.inner.treasury_transaction_count as u64)
                .add_field("milestone_count", m.inner.milestone_count as u64)
                .add_field("tagged_data_count", m.inner.tagged_data_count as u64)
                .add_field("no_payload_count", m.inner.no_payload_count as u64)
                .add_field("confirmed_count", m.inner.confirmed_count as u64)
                .add_field("conflicting_count", m.inner.conflicting_count as u64)
                .add_field("no_transaction_count", m.inner.no_transaction_count as u64),
            Measurement::DailyActiveAddresses(t) => t
                .prepare_query("stardust_daily_active_addresses")
                .add_field("count", t.inner.count as u64),
            Measurement::LedgerOutputs(m) => m
                .prepare_query("stardust_ledger_outputs")
                .add_field("basic_count", m.inner.basic.count as u64)
                .add_field("basic_value", m.inner.basic.value)
                .add_field("alias_count", m.inner.alias.count as u64)
                .add_field("alias_value", m.inner.alias.value)
                .add_field("foundry_count", m.inner.foundry.count as u64)
                .add_field("foundry_value", m.inner.foundry.value)
                .add_field("nft_count", m.inner.nft.count as u64)
                .add_field("nft_value", m.inner.nft.value)
                .add_field("treasury_count", m.inner.treasury.count as u64)
                .add_field("treasury_value", m.inner.treasury.value),
            Measurement::LedgerSize(m) => m
                .prepare_query("stardust_ledger_size")
                .add_field("total_key_bytes", m.inner.total_key_bytes)
                .add_field("total_data_bytes", m.inner.total_data_bytes)
                .add_field("total_storage_deposit_value", m.inner.total_storage_deposit_value),
            Measurement::MilestoneSize(m) => m
                .prepare_query("stardust_milestone_size")
                .add_field(
                    "total_milestone_payload_bytes",
                    m.inner.total_milestone_payload_bytes as u64,
                )
                .add_field(
                    "total_tagged_data_payload_bytes",
                    m.inner.total_tagged_data_payload_bytes as u64,
                )
                .add_field(
                    "total_transaction_payload_bytes",
                    m.inner.total_transaction_payload_bytes as u64,
                )
                .add_field(
                    "total_treasury_transaction_payload_bytes",
                    m.inner.total_treasury_transaction_payload_bytes as u64,
                )
                .add_field("total_milestone_bytes", m.inner.total_milestone_bytes as u64),
            Measurement::OutputActivity(m) => m
                .prepare_query("stardust_output_activity")
                .add_field("alias_created_count", m.inner.alias.created_count as u64)
                .add_field("alias_state_changed_count", m.inner.alias.state_changed_count as u64)
                .add_field(
                    "alias_governor_changed_count",
                    m.inner.alias.governor_changed_count as u64,
                )
                .add_field("alias_destroyed_count", m.inner.alias.destroyed_count as u64)
                .add_field("nft_created_count", m.inner.nft.created_count as u64)
                .add_field("nft_transferred_count", m.inner.nft.transferred_count as u64)
                .add_field("nft_destroyed_count", m.inner.nft.destroyed_count as u64)
                .add_field("foundry_created_count", m.inner.foundry.created_count as u64)
                .add_field("foundry_transferred_count", m.inner.foundry.transferred_count as u64)
                .add_field("foundry_destroyed_count", m.inner.foundry.destroyed_count as u64),
            Measurement::ProtocolParameters(m) => m
                .prepare_query("stardust_protocol_params")
                .add_field("token_supply", m.inner.token_supply)
                .add_field("min_pow_score", m.inner.min_pow_score)
                .add_field("below_max_depth", m.inner.below_max_depth)
                .add_field("v_byte_cost", m.inner.rent_structure.v_byte_cost)
                .add_field("v_byte_factor_key", m.inner.rent_structure.v_byte_factor_key)
                .add_field("v_byte_factor_data", m.inner.rent_structure.v_byte_factor_data),
            Measurement::UnclaimedTokens(m) => m
                .prepare_query("stardust_unclaimed_rewards")
                .add_field("unclaimed_count", m.inner.unclaimed_count as u64)
                .add_field("unclaimed_value", m.inner.unclaimed_value),
            Measurement::UnlockConditions(m) => m
                .prepare_query("stardust_unlock_conditions")
                .add_field("expiration_count", m.inner.expiration.count as u64)
                .add_field("expiration_value", m.inner.expiration.value)
                .add_field("timelock_count", m.inner.timelock.count as u64)
                .add_field("timelock_value", m.inner.timelock.value)
                .add_field(
                    "storage_deposit_return_count",
                    m.inner.storage_deposit_return.count as u64,
                )
                .add_field("storage_deposit_return_value", m.inner.storage_deposit_return.value)
                .add_field(
                    "storage_deposit_return_inner_value",
                    m.inner.storage_deposit_return_inner_value,
                ),
        }
    }
}
