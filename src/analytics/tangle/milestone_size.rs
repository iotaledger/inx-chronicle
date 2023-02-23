// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

/// Milestone size statistics.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct MilestoneSizeMeasurement {
    pub(crate) total_milestone_payload_bytes: usize,
    pub(crate) total_tagged_data_payload_bytes: usize,
    pub(crate) total_transaction_payload_bytes: usize,
    pub(crate) total_treasury_transaction_payload_bytes: usize,
    pub(crate) total_milestone_bytes: usize,
}

impl Analytics for MilestoneSizeMeasurement {
    type Measurement = Self;

    fn handle_block(&mut self, BlockData { block, raw, .. }: &BlockData, _ctx: &dyn AnalyticsContext) {
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

    fn end_milestone(&mut self, _ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        Some(std::mem::take(self))
    }
}
