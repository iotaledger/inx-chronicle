// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::BlockAnalytics;
use crate::types::{
    ledger::BlockMetadata,
    stardust::block::{Block, Payload},
    tangle::MilestoneIndex,
};

/// Milestone size statistics.
#[derive(Clone, Debug, Default)]
pub struct MilestoneSizeMeasurement {
    total_milestone_payload_bytes: usize,
    total_tagged_data_payload_bytes: usize,
    total_transaction_payload_bytes: usize,
    total_treasury_transaction_payload_bytes: usize,
    total_milestone_bytes: usize,
}

/// Computes the total and per-payload byte sizes for a given milestone.
pub struct MilestoneSizeAnalytics {
    measurement: MilestoneSizeMeasurement,
}

impl BlockAnalytics for MilestoneSizeAnalytics {
    type Measurement = MilestoneSizeMeasurement;

    fn begin_milestone(&mut self, _: MilestoneIndex) {
        self.measurement = MilestoneSizeMeasurement::default();
    }

    fn handle_block(&mut self, block: &Block, raw_block: &[u8], _: &BlockMetadata) {
        self.measurement.total_milestone_bytes += raw_block.len();
        match block.payload {
            Some(Payload::Milestone(_)) => self.measurement.total_milestone_payload_bytes += raw_block.len(),
            Some(Payload::TaggedData(_)) => self.measurement.total_tagged_data_payload_bytes += raw_block.len(),
            Some(Payload::Transaction(_)) => self.measurement.total_transaction_payload_bytes += raw_block.len(),
            Some(Payload::TreasuryTransaction(_)) => {
                self.measurement.total_treasury_transaction_payload_bytes += raw_block.len();
            }
            _ => {}
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement.clone())
    }
}
