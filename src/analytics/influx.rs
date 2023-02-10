// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Influx Measurement implementations

use influxdb::{InfluxDbWriteable, WriteQuery};
use time::Duration;

use super::{
    ledger::{
        AddressActivityMeasurement, AddressBalanceMeasurement, BaseTokenActivityMeasurement, LedgerOutputMeasurement,
        LedgerSizeMeasurement, OutputActivityMeasurement, UnclaimedTokenMeasurement, UnlockConditionMeasurement,
    },
    tangle::{BlockActivityMeasurement, MilestoneSizeMeasurement},
    PerMilestone, TimeInterval,
};
use crate::{
    db::influxdb::InfluxDb,
    types::{stardust::milestone::MilestoneTimestamp, tangle::ProtocolParameters},
};

/// A trait that defines an InfluxDb measurement.
trait Measurement {
    const NAME: &'static str;

    fn add_fields(&self, query: WriteQuery) -> WriteQuery;
}

trait AddFields<M: Measurement> {
    fn add_fields(self, measurement: &M) -> Self;
}

impl<M: Measurement> AddFields<M> for WriteQuery {
    fn add_fields(self, measurement: &M) -> Self {
        measurement.add_fields(self)
    }
}

pub(super) trait PrepareQuery: Send + Sync {
    fn prepare_query(&self) -> WriteQuery;
}

impl<T: PrepareQuery + ?Sized> PrepareQuery for Box<T> {
    fn prepare_query(&self) -> WriteQuery {
        (**self).prepare_query()
    }
}

impl<M: Send + Sync> PrepareQuery for PerMilestone<M>
where
    M: Measurement,
{
    fn prepare_query(&self) -> WriteQuery {
        influxdb::Timestamp::from(self.at.milestone_timestamp)
            .into_query(M::NAME)
            .add_field("milestone_index", self.at.milestone_index)
            .add_fields(&self.inner)
    }
}

impl<M: Send + Sync> PrepareQuery for TimeInterval<M>
where
    M: Measurement,
{
    fn prepare_query(&self) -> WriteQuery {
        // We subtract 1 nanosecond to get the inclusive end of the time interval.
        let timestamp = self.to_exclusive - Duration::nanoseconds(1);
        influxdb::Timestamp::from(MilestoneTimestamp::from(timestamp))
            .into_query(M::NAME)
            .add_fields(&self.inner)
    }
}

impl Measurement for AddressBalanceMeasurement {
    const NAME: &'static str = "stardust_addresses";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        let mut query = query.add_field("address_with_balance_count", self.address_with_balance_count as u64);
        for (index, stat) in self.distribution.iter() {
            query = query
                .add_field(format!("address_count_{index}"), stat.address_count)
                .add_field(format!("total_amount_{index}"), stat.total_amount.0);
        }
        query
    }
}

impl Measurement for BaseTokenActivityMeasurement {
    const NAME: &'static str = "stardust_base_token_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("booked_value", self.booked_amount.0)
            .add_field("transferred_value", self.transferred_amount.0)
    }
}

impl Measurement for BlockActivityMeasurement {
    const NAME: &'static str = "stardust_block_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("transaction_count", self.transaction_count as u64)
            .add_field("treasury_transaction_count", self.treasury_transaction_count as u64)
            .add_field("milestone_count", self.milestone_count as u64)
            .add_field("tagged_data_count", self.tagged_data_count as u64)
            .add_field("no_payload_count", self.no_payload_count as u64)
            .add_field("confirmed_count", self.confirmed_count as u64)
            .add_field("conflicting_count", self.conflicting_count as u64)
            .add_field("no_transaction_count", self.no_transaction_count as u64)
    }
}

impl Measurement for AddressActivityMeasurement {
    const NAME: &'static str = "stardust_daily_active_addresses";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query.add_field("count", self.count as u64)
    }
}

impl Measurement for LedgerOutputMeasurement {
    const NAME: &'static str = "stardust_ledger_outputs";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("basic_count", self.basic.count as u64)
            .add_field("basic_value", self.basic.amount.0)
            .add_field("alias_count", self.alias.count as u64)
            .add_field("alias_value", self.alias.amount.0)
            .add_field("foundry_count", self.foundry.count as u64)
            .add_field("foundry_value", self.foundry.amount.0)
            .add_field("nft_count", self.nft.count as u64)
            .add_field("nft_value", self.nft.amount.0)
            .add_field("treasury_count", self.treasury.count as u64)
            .add_field("treasury_value", self.treasury.amount.0)
    }
}

impl Measurement for LedgerSizeMeasurement {
    const NAME: &'static str = "stardust_ledger_size";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("total_key_bytes", self.total_key_bytes)
            .add_field("total_data_bytes", self.total_data_bytes)
            .add_field("total_storage_deposit_value", self.total_storage_deposit_value.0)
    }
}

impl Measurement for MilestoneSizeMeasurement {
    const NAME: &'static str = "stardust_milestone_size";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field(
                "total_milestone_payload_bytes",
                self.total_milestone_payload_bytes as u64,
            )
            .add_field(
                "total_tagged_data_payload_bytes",
                self.total_tagged_data_payload_bytes as u64,
            )
            .add_field(
                "total_transaction_payload_bytes",
                self.total_transaction_payload_bytes as u64,
            )
            .add_field(
                "total_treasury_transaction_payload_bytes",
                self.total_treasury_transaction_payload_bytes as u64,
            )
            .add_field("total_milestone_bytes", self.total_milestone_bytes as u64)
    }
}

impl Measurement for OutputActivityMeasurement {
    const NAME: &'static str = "stardust_output_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("alias_created_count", self.alias.created_count as u64)
            .add_field("alias_state_changed_count", self.alias.state_changed_count as u64)
            .add_field("alias_governor_changed_count", self.alias.governor_changed_count as u64)
            .add_field("alias_destroyed_count", self.alias.destroyed_count as u64)
            .add_field("nft_created_count", self.nft.created_count as u64)
            .add_field("nft_transferred_count", self.nft.transferred_count as u64)
            .add_field("nft_destroyed_count", self.nft.destroyed_count as u64)
            .add_field("foundry_created_count", self.foundry.created_count as u64)
            .add_field("foundry_transferred_count", self.foundry.transferred_count as u64)
            .add_field("foundry_destroyed_count", self.foundry.destroyed_count as u64)
    }
}

impl Measurement for ProtocolParameters {
    const NAME: &'static str = "stardust_protocol_params";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("token_supply", self.token_supply)
            .add_field("min_pow_score", self.min_pow_score)
            .add_field("below_max_depth", self.below_max_depth)
            .add_field("v_byte_cost", self.rent_structure.v_byte_cost)
            .add_field("v_byte_factor_key", self.rent_structure.v_byte_factor_key)
            .add_field("v_byte_factor_data", self.rent_structure.v_byte_factor_data)
    }
}

impl Measurement for UnclaimedTokenMeasurement {
    const NAME: &'static str = "stardust_unclaimed_rewards";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("unclaimed_count", self.unclaimed_count as u64)
            .add_field("unclaimed_value", self.unclaimed_value.0)
    }
}

impl Measurement for UnlockConditionMeasurement {
    const NAME: &'static str = "stardust_unlock_conditions";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("expiration_count", self.expiration.count as u64)
            .add_field("expiration_value", self.expiration.amount.0)
            .add_field("timelock_count", self.timelock.count as u64)
            .add_field("timelock_value", self.timelock.amount.0)
            .add_field("storage_deposit_return_count", self.storage_deposit_return.count as u64)
            .add_field("storage_deposit_return_value", self.storage_deposit_return.amount.0)
            .add_field(
                "storage_deposit_return_inner_value",
                self.storage_deposit_return_inner_value,
            )
    }
}

impl InfluxDb {
    /// Writes a [`Measurement`] to the InfluxDB database.
    pub(super) async fn insert_measurement(&self, measurement: impl PrepareQuery) -> Result<(), influxdb::Error> {
        self.analytics().query(measurement.prepare_query()).await?;
        Ok(())
    }
}
