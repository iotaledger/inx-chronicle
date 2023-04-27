// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Influx Measurement implementations

use influxdb::{InfluxDbWriteable, WriteQuery};

use super::{
    ledger::{
        AddressActivityMeasurement, AddressBalanceMeasurement, AddressesWithBalanceMeasurement,
        BaseTokenActivityMeasurement, LedgerOutputMeasurement, LedgerSizeMeasurement, OutputActivityMeasurement,
        TransactionSizeMeasurement, UnclaimedTokenMeasurement, UnlockConditionMeasurement,
    },
    tangle::{BlockActivityMeasurement, MilestoneSizeMeasurement},
    AnalyticsInterval, PerInterval, PerMilestone,
};
use crate::{
    db::influxdb::InfluxDb,
    model::{payload::milestone::MilestoneIndexTimestamp, ProtocolParameters},
};

/// A trait that defines an InfluxDb measurement.
trait Measurement {
    const NAME: &'static str;

    fn add_fields(&self, query: WriteQuery) -> WriteQuery;
}

impl<M: Measurement + ?Sized> Measurement for &M {
    const NAME: &'static str = M::NAME;

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        (*self).add_fields(query)
    }
}

/// A trait that defines an InfluxDb measurement over an interval.
trait IntervalMeasurement: Measurement {
    fn name(interval: AnalyticsInterval) -> String;
}

trait AddFields<M: Measurement> {
    fn add_fields(self, measurement: &M) -> Self;
}

impl<M: Measurement> AddFields<M> for WriteQuery {
    fn add_fields(self, measurement: &M) -> Self {
        measurement.add_fields(self)
    }
}

pub trait PerMilestoneQuery: Send + Sync {
    fn per_milestone_query(&self, at: MilestoneIndexTimestamp) -> Vec<WriteQuery>;
}

impl PerMilestoneQuery for Box<dyn PerMilestoneQuery> {
    fn per_milestone_query(&self, at: MilestoneIndexTimestamp) -> Vec<WriteQuery> {
        (&**self).per_milestone_query(at)
    }
}

impl<M: Measurement + Send + Sync> PerMilestoneQuery for M {
    fn per_milestone_query(&self, at: MilestoneIndexTimestamp) -> Vec<WriteQuery> {
        vec![
            influxdb::Timestamp::from(at.milestone_timestamp)
                .into_query(M::NAME)
                .add_field("milestone_index", at.milestone_index)
                .add_fields(self),
        ]
    }
}

impl<T: PerMilestoneQuery + Send + Sync> PerMilestoneQuery for Vec<T> {
    fn per_milestone_query(&self, at: MilestoneIndexTimestamp) -> Vec<WriteQuery> {
        self.iter().flat_map(|inner| inner.per_milestone_query(at)).collect()
    }
}

impl<T: PerMilestoneQuery + Send + Sync> PerMilestoneQuery for Option<T> {
    fn per_milestone_query(&self, at: MilestoneIndexTimestamp) -> Vec<WriteQuery> {
        self.iter().flat_map(|inner| inner.per_milestone_query(at)).collect()
    }
}

macro_rules! impl_per_milestone_query_tuple {
    ($($idx:tt $t:tt),+) => {
        impl<$($t: PerMilestoneQuery + Send + Sync),*> PerMilestoneQuery for ($($t),*,)
        {
            fn per_milestone_query(&self, at: MilestoneIndexTimestamp) -> Vec<WriteQuery> {
                let mut queries = Vec::new();
                $(
                    queries.extend(self.$idx.per_milestone_query(at));
                )*
                queries
            }
        }
    };
}
// Right now we only need this one
impl_per_milestone_query_tuple!(0 T0, 1 T1);

pub(crate) trait PrepareQuery: Send + Sync {
    fn prepare_query(&self) -> Vec<WriteQuery>;
}

impl<T: PrepareQuery + ?Sized> PrepareQuery for Box<T> {
    fn prepare_query(&self) -> Vec<WriteQuery> {
        (**self).prepare_query()
    }
}

impl<T: PerMilestoneQuery + Send + Sync> PrepareQuery for PerMilestone<T> {
    fn prepare_query(&self) -> Vec<WriteQuery> {
        self.inner.per_milestone_query(self.at)
    }
}

impl<M: Send + Sync> PrepareQuery for PerInterval<M>
where
    M: IntervalMeasurement,
{
    fn prepare_query(&self) -> Vec<WriteQuery> {
        vec![
            influxdb::Timestamp::Seconds(self.start_date.midnight().assume_utc().unix_timestamp() as _)
                .into_query(M::name(self.interval))
                .add_fields(&self.inner),
        ]
    }
}

impl Measurement for AddressesWithBalanceMeasurement {
    const NAME: &'static str = "stardust_addresses_with_balance";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        let mut query = query.add_field("address_with_balance_count", self.address_with_balance_count as u64);
        for (index, stat) in self.token_distribution.iter().enumerate() {
            query = query
                .add_field(format!("address_count_{index}"), stat.address_count)
                .add_field(format!("total_amount_{index}"), stat.total_amount.0);
        }
        query
    }
}

impl Measurement for AddressBalanceMeasurement {
    const NAME: &'static str = "stardust_address_balance";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_tag("address", self.address.clone())
            .add_field("balance", self.balance.0)
    }
}

impl Measurement for BaseTokenActivityMeasurement {
    const NAME: &'static str = "stardust_base_token_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("booked_amount", self.booked_amount.0)
            .add_field("transferred_amount", self.transferred_amount.0)
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
    const NAME: &'static str = "stardust_active_addresses";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query.add_field("count", self.count as u64)
    }
}

impl IntervalMeasurement for AddressActivityMeasurement {
    fn name(interval: AnalyticsInterval) -> String {
        format!("stardust_{interval}_active_addresses")
    }
}

impl Measurement for TransactionSizeMeasurement {
    const NAME: &'static str = "stardust_transaction_size_distribution";

    fn add_fields(&self, mut query: WriteQuery) -> WriteQuery {
        for (bucket, value) in self.input_buckets.single_buckets() {
            query = query.add_field(format!("input_{bucket}"), value as u64);
        }
        query = query
            .add_field("input_small", self.input_buckets.small as u64)
            .add_field("input_medium", self.input_buckets.medium as u64)
            .add_field("input_large", self.input_buckets.large as u64)
            .add_field("input_huge", self.input_buckets.huge as u64);
        for (bucket, value) in self.output_buckets.single_buckets() {
            query = query.add_field(format!("output_{bucket}"), value as u64);
        }
        query = query
            .add_field("output_small", self.output_buckets.small as u64)
            .add_field("output_medium", self.output_buckets.medium as u64)
            .add_field("output_large", self.output_buckets.large as u64)
            .add_field("output_huge", self.output_buckets.huge as u64);
        query
    }
}

impl Measurement for LedgerOutputMeasurement {
    const NAME: &'static str = "stardust_ledger_outputs";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("basic_count", self.basic.count as u64)
            .add_field("basic_amount", self.basic.amount.0)
            .add_field("alias_count", self.alias.count as u64)
            .add_field("alias_amount", self.alias.amount.0)
            .add_field("foundry_count", self.foundry.count as u64)
            .add_field("foundry_amount", self.foundry.amount.0)
            .add_field("nft_count", self.nft.count as u64)
            .add_field("nft_amount", self.nft.amount.0)
            .add_field("treasury_count", self.treasury.count as u64)
            .add_field("treasury_amount", self.treasury.amount.0)
    }
}

impl Measurement for LedgerSizeMeasurement {
    const NAME: &'static str = "stardust_ledger_size";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("total_key_bytes", self.total_key_bytes)
            .add_field("total_data_bytes", self.total_data_bytes)
            .add_field("total_storage_deposit_amount", self.total_storage_deposit_amount.0)
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
            .add_field("unclaimed_amount", self.unclaimed_amount.0)
    }
}

impl Measurement for UnlockConditionMeasurement {
    const NAME: &'static str = "stardust_unlock_conditions";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("expiration_count", self.expiration.count as u64)
            .add_field("expiration_amount", self.expiration.amount.0)
            .add_field("timelock_count", self.timelock.count as u64)
            .add_field("timelock_amount", self.timelock.amount.0)
            .add_field("storage_deposit_return_count", self.storage_deposit_return.count as u64)
            .add_field("storage_deposit_return_amount", self.storage_deposit_return.amount.0)
            .add_field(
                "storage_deposit_return_inner_amount",
                self.storage_deposit_return_inner_amount,
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
