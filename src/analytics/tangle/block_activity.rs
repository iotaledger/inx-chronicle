// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::{
    api::core::BlockState,
    block::{payload::Payload, BlockId, SignedBlock},
};

use super::*;
use crate::inx::responses::BlockMetadata;

/// The type of payloads that occured within a single milestone.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct BlockActivityMeasurement {
    pub(crate) no_payload_count: usize,
    pub(crate) tagged_data_count: usize,
    pub(crate) transaction_count: usize,
    pub(crate) candidacy_announcement_count: usize,
    pub(crate) pending_count: usize,
    pub(crate) confirmed_count: usize,
    pub(crate) finalized_count: usize,
    pub(crate) rejected_count: usize,
    pub(crate) failed_count: usize,
}

impl Analytics for BlockActivityMeasurement {
    type Measurement = Self;

    fn handle_block(
        &mut self,
        _block_id: BlockId,
        block: &SignedBlock,
        metadata: &BlockMetadata,
        _ctx: &dyn AnalyticsContext,
    ) {
        match block.block().as_basic_opt().and_then(|b| b.payload()) {
            Some(Payload::TaggedData(_)) => self.tagged_data_count += 1,
            Some(Payload::SignedTransaction(_)) => self.transaction_count += 1,
            Some(Payload::CandidacyAnnouncement(_)) => self.candidacy_announcement_count += 1,
            None => self.no_payload_count += 1,
        }
        match metadata.block_state {
            BlockState::Pending => self.pending_count += 1,
            BlockState::Confirmed => self.confirmed_count += 1,
            BlockState::Finalized => self.finalized_count += 1,
            BlockState::Rejected => self.rejected_count += 1,
            BlockState::Failed => self.failed_count += 1,
        }
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        std::mem::take(self)
    }
}
