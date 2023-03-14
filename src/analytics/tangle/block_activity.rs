// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use influxdb::WriteQuery;

use super::*;
use crate::{analytics::measurement::Measurement, model::metadata::LedgerInclusionState};

/// The type of payloads that occured within a single milestone.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct BlockActivityMeasurement {
    pub(crate) milestone_count: usize,
    pub(crate) no_payload_count: usize,
    pub(crate) tagged_data_count: usize,
    pub(crate) transaction_count: usize,
    pub(crate) treasury_transaction_count: usize,
    pub(crate) confirmed_count: usize,
    pub(crate) conflicting_count: usize,
    pub(crate) no_transaction_count: usize,
}

impl Analytics for BlockActivityMeasurement {
    type Measurement = Self;

    fn handle_block(&mut self, BlockData { block, metadata, .. }: &BlockData, _ctx: &dyn AnalyticsContext) {
        match block.payload {
            Some(Payload::Milestone(_)) => self.milestone_count += 1,
            Some(Payload::TaggedData(_)) => self.tagged_data_count += 1,
            Some(Payload::Transaction(_)) => self.transaction_count += 1,
            Some(Payload::TreasuryTransaction(_)) => self.treasury_transaction_count += 1,
            None => self.no_payload_count += 1,
        }
        match metadata.inclusion_state {
            LedgerInclusionState::Conflicting => self.conflicting_count += 1,
            LedgerInclusionState::Included => self.confirmed_count += 1,
            LedgerInclusionState::NoTransaction => self.no_transaction_count += 1,
        }
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        std::mem::take(self)
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
