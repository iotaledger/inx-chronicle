// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::types::ledger::LedgerInclusionState;

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
    type Measurement = PerMilestone<Self>;

    fn begin_milestone(&mut self, _at: MilestoneIndexTimestamp, _params: &ProtocolParameters) {
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

    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(PerMilestone { at, inner: *self })
    }
}
