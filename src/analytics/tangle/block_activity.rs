// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{payload::Payload, Block, BlockBody};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::block_metadata::{BlockMetadata, BlockState, TransactionState},
};

/// The type of payloads that occured within a single slot.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct BlockActivityMeasurement {
    pub(crate) basic_count: usize,
    pub(crate) validation_count: usize,
    pub(crate) no_payload_count: usize,
    pub(crate) tagged_data_count: usize,
    pub(crate) transaction_count: usize,
    pub(crate) candidacy_announcement_count: usize,
    pub(crate) block_pending_count: usize,
    pub(crate) block_accepted_count: usize,
    pub(crate) block_confirmed_count: usize,
    pub(crate) block_finalized_count: usize,
    pub(crate) block_rejected_count: usize,
    pub(crate) block_failed_count: usize,
    pub(crate) block_unknown_count: usize,
    pub(crate) txn_pending_count: usize,
    pub(crate) txn_accepted_count: usize,
    pub(crate) txn_confirmed_count: usize,
    pub(crate) txn_finalized_count: usize,
    pub(crate) txn_failed_count: usize,
}

#[async_trait::async_trait]
impl Analytics for BlockActivityMeasurement {
    type Measurement = Self;

    async fn handle_block(
        &mut self,
        block: &Block,
        metadata: &BlockMetadata,
        _ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        match block.body() {
            BlockBody::Basic(_) => self.basic_count += 1,
            BlockBody::Validation(_) => self.validation_count += 1,
        }
        match block.body().as_basic_opt().and_then(|b| b.payload()) {
            Some(Payload::TaggedData(_)) => self.tagged_data_count += 1,
            Some(Payload::SignedTransaction(_)) => self.transaction_count += 1,
            Some(Payload::CandidacyAnnouncement(_)) => self.candidacy_announcement_count += 1,
            None => self.no_payload_count += 1,
        }
        match &metadata.block_state {
            BlockState::Pending => self.block_pending_count += 1,
            BlockState::Accepted => self.block_accepted_count += 1,
            BlockState::Confirmed => self.block_confirmed_count += 1,
            BlockState::Finalized => self.block_finalized_count += 1,
            BlockState::Rejected => self.block_rejected_count += 1,
            BlockState::Failed => self.block_failed_count += 1,
            BlockState::Unknown => self.block_unknown_count += 1,
        }
        if let Some(txn_state) = metadata.transaction_metadata.as_ref().map(|m| &m.transaction_state) {
            match txn_state {
                TransactionState::Pending => self.txn_pending_count += 1,
                TransactionState::Accepted => self.txn_accepted_count += 1,
                TransactionState::Confirmed => self.txn_confirmed_count += 1,
                TransactionState::Finalized => self.txn_finalized_count += 1,
                TransactionState::Failed => self.txn_failed_count += 1,
            }
        }

        Ok(())
    }

    async fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        Ok(std::mem::take(self))
    }
}
