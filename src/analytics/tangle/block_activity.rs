// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::BlockAnalytics;
use crate::{
    tangle::BlockData,
    types::{ledger::LedgerInclusionState, stardust::block::Payload, tangle::MilestoneIndex},
};

/// The type of payloads that occured within a single milestone.
#[derive(Copy, Clone, Debug, Default)]
pub struct BlockActivityMeasurement {
    pub milestone_count: usize,
    pub no_payload_count: usize,
    pub tagged_data_count: usize,
    pub transaction_count: usize,
    pub treasury_transaction_count: usize,
    pub confirmed_count: usize,
    pub conflicting_count: usize,
    pub no_transaction_count: usize,
}

impl BlockAnalytics for BlockActivityMeasurement {
    type Measurement = Self;

    fn begin_milestone(&mut self, _: MilestoneIndex) {
        *self = Default::default();
    }

    fn handle_block(&mut self, BlockData { block, metadata, .. }: &BlockData) {
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

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self> {
        Some(*self)
    }
}
