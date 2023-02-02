// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::BlockAnalytics;
use crate::{
    tangle::BlockData,
    types::{stardust::block::Payload, tangle::MilestoneIndex},
};

/// Milestone size statistics.
#[derive(Copy, Clone, Debug, Default)]
pub struct MilestoneSizeMeasurement {
    pub total_milestone_payload_bytes: usize,
    pub total_tagged_data_payload_bytes: usize,
    pub total_transaction_payload_bytes: usize,
    pub total_treasury_transaction_payload_bytes: usize,
    pub total_milestone_bytes: usize,
}

impl BlockAnalytics for MilestoneSizeMeasurement {
    type Measurement = Self;

    fn begin_milestone(&mut self, _: MilestoneIndex) {
        *self = Self::default();
    }

    fn handle_block(&mut self, BlockData { block, raw, .. }: &BlockData) {
        self.total_milestone_bytes += raw.len();
        match block.payload {
            Some(Payload::Milestone(_)) => self.total_milestone_payload_bytes += raw.len(),
            Some(Payload::TaggedData(_)) => self.total_tagged_data_payload_bytes += raw.len(),
            Some(Payload::Transaction(_)) => self.total_transaction_payload_bytes += raw.len(),
            Some(Payload::TreasuryTransaction(_)) => {
                self.total_treasury_transaction_payload_bytes += raw.len();
            }
            _ => {}
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self> {
        Some(*self)
    }
}
