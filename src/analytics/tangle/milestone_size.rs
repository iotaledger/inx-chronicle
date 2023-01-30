// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::BlockAnalytics;
use crate::{
    db::collections::analytics::MilestoneSizeAnalyticsResult,
    tangle::BlockData,
    types::{stardust::block::Payload, tangle::MilestoneIndex},
};

/// Milestone size statistics.
#[derive(Clone, Debug, Default)]
pub struct MilestoneSizeAnalytics {
    total_milestone_payload_bytes: usize,
    total_tagged_data_payload_bytes: usize,
    total_transaction_payload_bytes: usize,
    total_treasury_transaction_payload_bytes: usize,
    total_milestone_bytes: usize,
}

impl BlockAnalytics for MilestoneSizeAnalytics {
    type Measurement = MilestoneSizeAnalyticsResult;

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

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(MilestoneSizeAnalyticsResult {
            total_milestone_payload_bytes: self.total_milestone_payload_bytes as _,
            total_tagged_data_payload_bytes: self.total_tagged_data_payload_bytes as _,
            total_transaction_payload_bytes: self.total_transaction_payload_bytes as _,
            total_treasury_transaction_payload_bytes: self.total_treasury_transaction_payload_bytes as _,
            total_milestone_bytes: self.total_milestone_bytes as _,
        })
    }
}
