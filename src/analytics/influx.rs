// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Influx Measurement implementations

use influxdb::{InfluxDbWriteable, WriteQuery};
use iota_sdk::types::block::protocol::ProtocolParameters;

use super::{
    ledger::{
        AddressActivityMeasurement, AddressBalanceMeasurement, BaseTokenActivityMeasurement, FeaturesMeasurement,
        LedgerOutputMeasurement, LedgerSizeMeasurement, OutputActivityMeasurement, TransactionSizeMeasurement,
        UnlockConditionMeasurement,
    },
    tangle::{
        BlockActivityMeasurement, BlockIssuerMeasurement, ManaActivityMeasurement, SlotCommitmentMeasurement,
        SlotSizeMeasurement,
    },
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
            influxdb::Timestamp::Seconds(self.slot_timestamp as _)
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
        let mut query = query
            .add_field(
                "ed25519_address_with_balance_count",
                self.ed25519_address_with_balance_count as u64,
            )
            .add_field(
                "account_address_with_balance_count",
                self.account_address_with_balance_count as u64,
            )
            .add_field(
                "nft_address_with_balance_count",
                self.nft_address_with_balance_count as u64,
            )
            .add_field(
                "anchor_address_with_balance_count",
                self.anchor_address_with_balance_count as u64,
            )
            .add_field(
                "implicit_account_address_with_balance_count",
                self.implicit_address_with_balance_count as u64,
            );
        for (index, stat) in self.token_distribution.iter().enumerate() {
            query = query
                .add_field(format!("ed25519_address_count_{index}"), stat.ed25519_count as u64)
                .add_field(format!("ed25519_total_amount_{index}"), stat.ed25519_amount)
                .add_field(format!("account_address_count_{index}"), stat.account_count as u64)
                .add_field(format!("account_total_amount_{index}"), stat.account_amount)
                .add_field(format!("nft_address_count_{index}"), stat.nft_count as u64)
                .add_field(format!("nft_total_amount_{index}"), stat.nft_amount)
                .add_field(format!("anchor_address_count_{index}"), stat.anchor_count as u64)
                .add_field(format!("anchor_total_amount_{index}"), stat.anchor_amount)
                .add_field(
                    format!("implicit_account_address_count_{index}"),
                    stat.implicit_count as u64,
                )
                .add_field(format!("implicit_account_total_amount_{index}"), stat.implicit_amount);
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
            .add_field("basic_count", self.basic_count as u64)
            .add_field("validation_count", self.validation_count as u64)
            .add_field("transaction_count", self.transaction_count as u64)
            .add_field("tagged_data_count", self.tagged_data_count as u64)
            .add_field("candidacy_announcement_count", self.candidacy_announcement_count as u64)
            .add_field("no_payload_count", self.no_payload_count as u64)
            .add_field("block_pending_count", self.block_pending_count as u64)
            .add_field("block_accepted_count", self.block_accepted_count as u64)
            .add_field("block_confirmed_count", self.block_confirmed_count as u64)
            .add_field("block_finalized_count", self.block_finalized_count as u64)
            .add_field("block_dropped_count", self.block_dropped_count as u64)
            .add_field("block_orphaned_count", self.block_orphaned_count as u64)
            .add_field("block_unknown_count", self.block_unknown_count as u64)
            .add_field("txn_pending_count", self.txn_pending_count as u64)
            .add_field("txn_accepted_count", self.txn_accepted_count as u64)
            .add_field("txn_committed_count", self.txn_committed_count as u64)
            .add_field("txn_finalized_count", self.txn_finalized_count as u64)
            .add_field("txn_failed_count", self.txn_failed_count as u64)
    }
}

impl Measurement for BlockIssuerMeasurement {
    const NAME: &'static str = "iota_block_issuer_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query.add_field("active_issuer_count", self.active_issuer_count as u64)
    }
}

impl Measurement for ManaActivityMeasurement {
    const NAME: &'static str = "iota_mana_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("rewards_claimed", self.rewards_claimed)
            .add_field("mana_burned", self.mana_burned)
            .add_field("bic_burned", self.bic_burned)
    }
}

impl Measurement for AddressActivityMeasurement {
    const NAME: &'static str = "iota_active_addresses";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("ed25519_count", self.ed25519_count as u64)
            .add_field("account_count", self.account_count as u64)
            .add_field("nft_count", self.nft_count as u64)
            .add_field("anchor_count", self.anchor_count as u64)
            .add_field("implicit_account_count", self.implicit_count as u64)
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
            .add_field("block_issuer_accounts", self.account.block_issuers_count as u64)
            .add_field("anchor_count", self.anchor.count as u64)
            .add_field("anchor_amount", self.anchor.amount)
            .add_field("foundry_count", self.foundry.count as u64)
            .add_field("foundry_amount", self.foundry.amount)
            .add_field("nft_count", self.nft.count as u64)
            .add_field("nft_amount", self.nft.amount)
            .add_field("delegation_count", self.delegation.count as u64)
            .add_field("delegation_amount", self.delegation.amount)
            .add_field("delegated_amount", self.delegation.delegated_amount)
    }
}

impl Measurement for LedgerSizeMeasurement {
    const NAME: &'static str = "iota_ledger_size";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query.add_field("total_storage_score", self.total_storage_score)
    }
}

impl Measurement for SlotCommitmentMeasurement {
    const NAME: &'static str = "iota_slot_commitment";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query.add_field("reference_mana_cost", self.reference_mana_cost)
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
            .add_field(
                "account_block_issuer_key_rotated_count",
                self.account.block_issuer_key_rotated as u64,
            )
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
            .add_field("delegation_delayed_count", self.delegation.delayed_count as u64)
            .add_field("delegation_destroyed_count", self.delegation.destroyed_count as u64)
            .add_field("native_token_minted_count", self.native_token.minted_count as u64)
            .add_field("native_token_melted_count", self.native_token.melted_count as u64)
    }
}

impl Measurement for ProtocolParameters {
    const NAME: &'static str = "iota_protocol_params";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        // TODO
        query.add_field("token_supply", self.token_supply())
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

impl Measurement for FeaturesMeasurement {
    const NAME: &'static str = "iota_features";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("native_tokens_count", self.native_tokens.count as u64)
            .add_field("native_tokens_amount", self.native_tokens.amount.to_string())
            .add_field("block_issuer_count", self.block_issuer.count as u64)
            .add_field("block_issuer_amount", self.block_issuer.amount)
            .add_field("staking_count", self.staking.count as u64)
            .add_field("staked_amount", self.staking.staked_amount)
    }
}

impl InfluxDb {
    /// Writes a [`Measurement`] to the InfluxDB database.
    pub(super) async fn insert_measurement(&self, measurement: impl PrepareQuery) -> Result<(), influxdb::Error> {
        self.analytics().query(measurement.prepare_query()).await?;
        Ok(())
    }
}
