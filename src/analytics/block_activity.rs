// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::BlockAnalytics;
use crate::types::{
    ledger::BlockMetadata,
    stardust::block::{Block, Payload},
    tangle::MilestoneIndex,
};

/// The type of payloads that occured within a single milestone.
#[derive(Clone, Debug, Default)]
pub struct BlockActivity {
    milestone_count: usize,
    no_payload_count: usize,
    tagged_data_count: usize,
    transaction_count: usize,
    treasury_transaction_count: usize,
}

/// Computes the block-level activity that happened in a milestone.
pub struct BlockActivityAnalytics {
    measurement: BlockActivity,
}

impl BlockAnalytics for BlockActivityAnalytics {
    type Measurement = BlockActivity;

    fn begin_milestone(&mut self, _: MilestoneIndex) {
        self.measurement = BlockActivity::default();
    }

    fn handle_block(&mut self, block: &Block, _: &BlockMetadata,) {
        match block.payload {
            Some(Payload::Milestone(_)) => self.measurement.milestone_count += 1,
            Some(Payload::TaggedData(_)) => self.measurement.tagged_data_count += 1,
            Some(Payload::Transaction(_)) => self.measurement.transaction_count += 1,
            Some(Payload::TreasuryTransaction(_)) => self.measurement.treasury_transaction_count += 1,
            None => self.measurement.no_payload_count += 1,
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement.clone())
    }
}
