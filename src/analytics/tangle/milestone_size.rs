// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

/// Milestone size statistics.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct MilestoneSizeMeasurement {
    pub(crate) total_milestone_payload_bytes: usize,
    pub(crate) total_tagged_data_payload_bytes: usize,
    pub(crate) total_transaction_payload_bytes: usize,
    pub(crate) total_treasury_transaction_payload_bytes: usize,
    pub(crate) total_milestone_bytes: usize,
}

impl Analytics for MilestoneSizeMeasurement {
    type Measurement = PerMilestone<Self>;

    fn begin_milestone(&mut self, _at: MilestoneIndexTimestamp, _params: &ProtocolParameters) {
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

    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(PerMilestone { at, inner: *self })
    }
}
