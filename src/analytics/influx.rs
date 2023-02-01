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

/// A trait that defines an InfluxDb measurement.
trait Measurement {
    const NAME: &'static str;

    fn add_fields(&self, query: WriteQuery) -> WriteQuery;
}

pub(crate) trait PrepareQuery: Send + Sync {
    fn prepare_query(&self) -> WriteQuery;
}

impl<T: PrepareQuery + ?Sized> PrepareQuery for Box<T> {
    fn prepare_query(&self) -> WriteQuery {
        (&**self).prepare_query()
    }
}

impl<M: Send + Sync> PrepareQuery for PerMilestone<M>
where
    Self: Measurement,
{
    fn prepare_query(&self) -> WriteQuery {
        influxdb::Timestamp::from(self.at.milestone_timestamp)
            .into_query(Self::NAME)
            .add_field("milestone_index", self.at.milestone_index)
    }
}

impl<M: Send + Sync> PrepareQuery for TimeInterval<M>
where
    Self: Measurement,
{
    fn prepare_query(&self) -> WriteQuery {
        // We subtract 1 nanosecond to get the inclusive end of the time interval.
        let timestamp = self.to_exclusive - Duration::nanoseconds(1);
        influxdb::Timestamp::from(MilestoneTimestamp::from(timestamp)).into_query(Self::NAME)
    }
}

impl Measurement for PerMilestone<AddressBalanceMeasurement> {
    const NAME: &'static str = "stardust_address_balances";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query.add_field(
            "address_with_balance_count",
            self.inner.address_with_balance_count as u64,
        )
    }
}

impl Measurement for PerMilestone<BaseTokenActivityMeasurement> {
    const NAME: &'static str = "stardust_base_token_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("booked_value", self.inner.booked_value)
            .add_field("transferred_value", self.inner.transferred_value)
    }
}

impl Measurement for PerMilestone<BlockActivityMeasurement> {
    const NAME: &'static str = "stardust_block_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("transaction_count", self.inner.transaction_count as u64)
            .add_field(
                "treasury_transaction_count",
                self.inner.treasury_transaction_count as u64,
            )
            .add_field("milestone_count", self.inner.milestone_count as u64)
            .add_field("tagged_data_count", self.inner.tagged_data_count as u64)
            .add_field("no_payload_count", self.inner.no_payload_count as u64)
            .add_field("confirmed_count", self.inner.confirmed_count as u64)
            .add_field("conflicting_count", self.inner.conflicting_count as u64)
            .add_field("no_transaction_count", self.inner.no_transaction_count as u64)
    }
}

impl Measurement for TimeInterval<AddressActivityMeasurement> {
    const NAME: &'static str = "stardust_daily_active_addresses";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query.add_field("count", self.inner.count as u64)
    }
}

impl Measurement for PerMilestone<LedgerOutputMeasurement> {
    const NAME: &'static str = "stardust_ledger_output";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("basic_count", self.inner.basic.count as u64)
            .add_field("basic_value", self.inner.basic.value)
            .add_field("alias_count", self.inner.alias.count as u64)
            .add_field("alias_value", self.inner.alias.value)
            .add_field("foundry_count", self.inner.foundry.count as u64)
            .add_field("foundry_value", self.inner.foundry.value)
            .add_field("nft_count", self.inner.nft.count as u64)
            .add_field("nft_value", self.inner.nft.value)
            .add_field("treasury_count", self.inner.treasury.count as u64)
            .add_field("treasury_value", self.inner.treasury.value)
    }
}

impl Measurement for PerMilestone<LedgerSizeMeasurement> {
    const NAME: &'static str = "stardust_ledger_size";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("total_key_bytes", self.inner.total_key_bytes)
            .add_field("total_data_bytes", self.inner.total_data_bytes)
            .add_field("total_storage_deposit_value", self.inner.total_storage_deposit_value)
    }
}

impl Measurement for PerMilestone<MilestoneSizeMeasurement> {
    const NAME: &'static str = "stardust_milestone_size";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field(
                "total_milestone_payload_bytes",
                self.inner.total_milestone_payload_bytes as u64,
            )
            .add_field(
                "total_tagged_data_payload_bytes",
                self.inner.total_tagged_data_payload_bytes as u64,
            )
            .add_field(
                "total_transaction_payload_bytes",
                self.inner.total_transaction_payload_bytes as u64,
            )
            .add_field(
                "total_treasury_transaction_payload_bytes",
                self.inner.total_treasury_transaction_payload_bytes as u64,
            )
            .add_field("total_milestone_bytes", self.inner.total_milestone_bytes as u64)
    }
}

impl Measurement for PerMilestone<OutputActivityMeasurement> {
    const NAME: &'static str = "stardust_output_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("alias_created_count", self.inner.alias.created_count as u64)
            .add_field("alias_state_changed_count", self.inner.alias.state_changed_count as u64)
            .add_field(
                "alias_governor_changed_count",
                self.inner.alias.governor_changed_count as u64,
            )
            .add_field("alias_destroyed_count", self.inner.alias.destroyed_count as u64)
            .add_field("nft_created_count", self.inner.nft.created_count as u64)
            .add_field("nft_transferred_count", self.inner.nft.transferred_count as u64)
            .add_field("nft_destroyed_count", self.inner.nft.destroyed_count as u64)
            .add_field("foundry_created_count", self.inner.foundry.created_count as u64)
            .add_field("foundry_transferred_count", self.inner.foundry.transferred_count as u64)
            .add_field("foundry_destroyed_count", self.inner.foundry.destroyed_count as u64)
    }
}

impl Measurement for PerMilestone<ProtocolParameters> {
    const NAME: &'static str = "stardust_protocol_params";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("token_supply", self.inner.token_supply)
            .add_field("min_pow_score", self.inner.min_pow_score)
            .add_field("below_max_depth", self.inner.below_max_depth)
            .add_field("v_byte_cost", self.inner.rent_structure.v_byte_cost)
            .add_field("v_byte_factor_key", self.inner.rent_structure.v_byte_factor_key)
            .add_field("v_byte_factor_data", self.inner.rent_structure.v_byte_factor_data)
    }
}

impl Measurement for PerMilestone<UnclaimedTokenMeasurement> {
    const NAME: &'static str = "stardust_unclaimed_tokens";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("unclaimed_count", self.inner.unclaimed_count as u64)
            .add_field("unclaimed_value", self.inner.unclaimed_value)
    }
}

impl Measurement for PerMilestone<UnlockConditionMeasurement> {
    const NAME: &'static str = "stardust_unlock_conditions";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("expiration_count", self.inner.expiration.count as u64)
            .add_field("expiration_value", self.inner.expiration.value)
            .add_field("timelock_count", self.inner.timelock.count as u64)
            .add_field("timelock_value", self.inner.timelock.value)
            .add_field(
                "storage_deposit_return_count",
                self.inner.storage_deposit_return.count as u64,
            )
            .add_field("storage_deposit_return_value", self.inner.storage_deposit_return.value)
            .add_field(
                "storage_deposit_return_inner_value",
                self.inner.storage_deposit_return_inner_value,
            )
    }
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct PerMilestone<M> {
    pub at: MilestoneIndexTimestamp,
    pub inner: M,
}

/// Note: We will need this later, for example for daily active addresses.
#[allow(unused)]
#[allow(missing_docs)]
pub struct TimeInterval<M> {
    pub from: OffsetDateTime,
    pub to_exclusive: OffsetDateTime,
    pub inner: M,
}

impl InfluxDb {
    /// Writes a [`Measurement`] to the InfluxDB database.
    pub(crate) async fn insert_measurement(&self, measurement: impl PrepareQuery) -> Result<(), influxdb::Error> {
        self.analytics().query(measurement.prepare_query()).await?;
        Ok(())
    }
}
