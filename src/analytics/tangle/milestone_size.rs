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
pub struct MilestoneSizeMeasurement {
    total_milestone_payload_bytes: u64,
    total_tagged_data_payload_bytes: u64,
    total_transaction_payload_bytes: u64,
    total_treasury_transaction_payload_bytes: u64,
    total_milestone_bytes: u64,
}

/// Computes the block-level activity that happened in a milestone.
pub struct MilestoneSizeAnalytics {
    measurement: MilestoneSizeMeasurement,
}

impl BlockAnalytics for MilestoneSizeAnalytics {
    type Measurement = MilestoneSizeMeasurement;

    fn begin_milestone(&mut self, _: MilestoneIndex) {
        self.measurement = MilestoneSizeMeasurement::default();
    }

    fn handle_block(&mut self, block: &Block, raw_block: Vec<u8>, _: &BlockMetadata) {
        self.measurement.total_milestone_bytes += raw_block.len() as u64;
        match block.payload {
            Some(Payload::Milestone(_)) => self.measurement.total_milestone_payload_bytes += raw_block.len() as u64,
            Some(Payload::TaggedData(_)) => self.measurement.total_tagged_data_payload_bytes += raw_block.len() as u64,
            Some(Payload::Transaction(_)) => self.measurement.total_transaction_payload_bytes += raw_block.len() as u64,
            Some(Payload::TreasuryTransaction(_)) => {
                self.measurement.total_treasury_transaction_payload_bytes += raw_block.len() as u64
            }
            _ => {}
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement.clone())
    }
}
