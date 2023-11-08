// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Influx Measurement implementations

use influxdb::{InfluxDbWriteable, WriteQuery};
use iota_sdk::types::block::protocol::ProtocolParameters;

use super::{
    ledger::{
        AddressActivityMeasurement, AddressBalanceMeasurement, BaseTokenActivityMeasurement, LedgerOutputMeasurement,
        LedgerSizeMeasurement, OutputActivityMeasurement, TransactionSizeMeasurement, UnclaimedTokenMeasurement,
        UnlockConditionMeasurement,
    },
    tangle::{BlockActivityMeasurement, SlotSizeMeasurement},
    AnalyticsInterval, PerInterval, PerSlot,
};
use crate::db::influxdb::InfluxDb;

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

pub trait PrepareQuery: Send + Sync {
    fn prepare_query(&self) -> Vec<WriteQuery>;
}

impl<T: PrepareQuery + ?Sized> PrepareQuery for Box<T> {
    fn prepare_query(&self) -> Vec<WriteQuery> {
        (**self).prepare_query()
    }
}

impl<M: Send + Sync> PrepareQuery for PerSlot<M>
where
    M: Measurement,
{
    fn prepare_query(&self) -> Vec<WriteQuery> {
        vec![
            influxdb::Timestamp::Nanoseconds(self.slot_timestamp as _)
                .into_query(M::NAME)
                .add_field("slot_index", self.slot_index.0)
                .add_fields(&self.inner),
        ]
    }
}

impl<T: PrepareQuery> PrepareQuery for PerSlot<Vec<T>> {
    fn prepare_query(&self) -> Vec<WriteQuery> {
        self.inner.iter().flat_map(|inner| inner.prepare_query()).collect()
    }
}

impl<M: Send + Sync> PrepareQuery for PerSlot<Option<M>>
where
    M: Measurement,
{
    fn prepare_query(&self) -> Vec<WriteQuery> {
        self.inner
            .iter()
            .flat_map(|inner| {
                PerSlot {
                    slot_timestamp: self.slot_timestamp,
                    slot_index: self.slot_index,
                    inner,
                }
                .prepare_query()
            })
            .collect()
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

impl Measurement for AddressBalanceMeasurement {
    const NAME: &'static str = "iota_addresses";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        let mut query = query.add_field("address_with_balance_count", self.address_with_balance_count as u64);
        for (index, stat) in self.token_distribution.iter().enumerate() {
            query = query
                .add_field(format!("address_count_{index}"), stat.address_count)
                .add_field(format!("total_amount_{index}"), stat.total_amount);
        }
        query
    }
}

impl Measurement for BaseTokenActivityMeasurement {
    const NAME: &'static str = "iota_base_token_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("booked_amount", self.booked_amount)
            .add_field("transferred_amount", self.transferred_amount)
    }
}

impl Measurement for BlockActivityMeasurement {
    const NAME: &'static str = "iota_block_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("transaction_count", self.transaction_count as u64)
            .add_field("tagged_data_count", self.tagged_data_count as u64)
            .add_field("candidacy_announcement_count", self.candidacy_announcement_count as u64)
            .add_field("no_payload_count", self.no_payload_count as u64)
            .add_field("confirmed_count", self.pending_count as u64)
            .add_field("confirmed_count", self.confirmed_count as u64)
            .add_field("finalized_count", self.finalized_count as u64)
            .add_field("rejected_count", self.rejected_count as u64)
            .add_field("failed_count", self.failed_count as u64)
    }
}

impl Measurement for AddressActivityMeasurement {
    const NAME: &'static str = "iota_active_addresses";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query.add_field("count", self.count as u64)
    }
}

impl IntervalMeasurement for AddressActivityMeasurement {
    fn name(interval: AnalyticsInterval) -> String {
        format!("iota_{interval}_active_addresses")
    }
}

impl Measurement for TransactionSizeMeasurement {
    const NAME: &'static str = "iota_transaction_size_distribution";

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
    const NAME: &'static str = "iota_ledger_outputs";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("basic_count", self.basic.count as u64)
            .add_field("basic_amount", self.basic.amount)
            .add_field("account_count", self.account.count as u64)
            .add_field("account_amount", self.account.amount)
            .add_field("anchor_count", self.anchor.count as u64)
            .add_field("anchor_amount", self.anchor.amount)
            .add_field("foundry_count", self.foundry.count as u64)
            .add_field("foundry_amount", self.foundry.amount)
            .add_field("nft_count", self.nft.count as u64)
            .add_field("nft_amount", self.nft.amount)
            .add_field("delegation_count", self.delegation.count as u64)
            .add_field("delegation_amount", self.delegation.amount)
    }
}

impl Measurement for LedgerSizeMeasurement {
    const NAME: &'static str = "iota_ledger_size";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query.add_field("total_storage_cost", self.total_storage_cost)
    }
}

impl Measurement for SlotSizeMeasurement {
    const NAME: &'static str = "iota_slot_size";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field(
                "total_tagged_data_payload_bytes",
                self.total_tagged_data_payload_bytes as u64,
            )
            .add_field(
                "total_transaction_payload_bytes",
                self.total_transaction_payload_bytes as u64,
            )
            .add_field(
                "total_candidacy_announcement_payload_bytes",
                self.total_candidacy_announcement_payload_bytes as u64,
            )
            .add_field("total_slot_bytes", self.total_slot_bytes as u64)
    }
}

impl Measurement for OutputActivityMeasurement {
    const NAME: &'static str = "iota_output_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("account_created_count", self.account.created_count as u64)
            .add_field("account_destroyed_count", self.account.destroyed_count as u64)
            .add_field("anchor_created_count", self.anchor.created_count as u64)
            .add_field("anchor_state_changed_count", self.anchor.state_changed_count as u64)
            .add_field(
                "anchor_governor_changed_count",
                self.anchor.governor_changed_count as u64,
            )
            .add_field("anchor_destroyed_count", self.anchor.destroyed_count as u64)
            .add_field("nft_created_count", self.nft.created_count as u64)
            .add_field("nft_transferred_count", self.nft.transferred_count as u64)
            .add_field("nft_destroyed_count", self.nft.destroyed_count as u64)
            .add_field("foundry_created_count", self.foundry.created_count as u64)
            .add_field("foundry_transferred_count", self.foundry.transferred_count as u64)
            .add_field("foundry_destroyed_count", self.foundry.destroyed_count as u64)
            .add_field("delegation_created_count", self.delegation.created_count as u64)
            .add_field("delegation_destroyed_count", self.delegation.destroyed_count as u64)
    }
}

impl Measurement for ProtocolParameters {
    const NAME: &'static str = "iota_protocol_params";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        // TODO
        query.add_field("token_supply", self.token_supply())
    }
}

impl Measurement for UnclaimedTokenMeasurement {
    const NAME: &'static str = "iota_unclaimed_rewards";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("unclaimed_count", self.unclaimed_count as u64)
            .add_field("unclaimed_amount", self.unclaimed_amount)
    }
}

impl Measurement for UnlockConditionMeasurement {
    const NAME: &'static str = "iota_unlock_conditions";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("expiration_count", self.expiration.count as u64)
            .add_field("expiration_amount", self.expiration.amount)
            .add_field("timelock_count", self.timelock.count as u64)
            .add_field("timelock_amount", self.timelock.amount)
            .add_field("storage_deposit_return_count", self.storage_deposit_return.count as u64)
            .add_field("storage_deposit_return_amount", self.storage_deposit_return.amount)
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
