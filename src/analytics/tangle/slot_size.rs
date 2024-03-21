// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{payload::Payload, Block};
use packable::PackableExt;

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::block_metadata::BlockMetadata,
};

/// Slot size statistics.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct SlotSizeMeasurement {
    pub(crate) total_tagged_data_payload_bytes: usize,
    pub(crate) total_transaction_payload_bytes: usize,
    pub(crate) total_candidacy_announcement_payload_bytes: usize,
    pub(crate) total_slot_bytes: usize,
}

#[async_trait::async_trait]
impl Analytics for SlotSizeMeasurement {
    type Measurement = Self;

    async fn handle_block(
        &mut self,
        block: &Block,
        _metadata: &BlockMetadata,
        _ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        let byte_len = block.packed_len();
        self.total_slot_bytes += byte_len;
        match block.body().as_basic_opt().and_then(|b| b.payload()) {
            Some(Payload::TaggedData(_)) => self.total_tagged_data_payload_bytes += byte_len,
            Some(Payload::SignedTransaction(_)) => self.total_transaction_payload_bytes += byte_len,
            Some(Payload::CandidacyAnnouncement(_)) => self.total_candidacy_announcement_payload_bytes += byte_len,
            _ => {}
        }
        Ok(())
    }

    async fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        Ok(std::mem::take(self))
    }
}
