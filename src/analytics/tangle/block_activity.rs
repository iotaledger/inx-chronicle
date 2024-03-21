// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::{
    api::core::{BlockState, TransactionState},
    block::{
        payload::{Payload, SignedTransactionPayload},
        Block, BlockBody,
    },
};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::{
        block_metadata::{BlockMetadata, TransactionMetadata},
        ledger::{LedgerOutput, LedgerSpent},
    },
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
    pub(crate) block_dropped_count: usize,
    pub(crate) block_orphaned_count: usize,
    pub(crate) block_unknown_count: usize,
    pub(crate) txn_pending_count: usize,
    pub(crate) txn_accepted_count: usize,
    pub(crate) txn_committed_count: usize,
    pub(crate) txn_finalized_count: usize,
    pub(crate) txn_failed_count: usize,
}

#[async_trait::async_trait]
impl Analytics for BlockActivityMeasurement {
    type Measurement = Self;

    async fn handle_block(
        &mut self,
        block: &Block,
        block_metadata: &BlockMetadata,
        _ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        match block.body() {
            BlockBody::Basic(basic_body) => {
                self.basic_count += 1;
                match basic_body.payload() {
                    Some(Payload::TaggedData(_)) => self.tagged_data_count += 1,
                    Some(Payload::SignedTransaction(_)) => self.transaction_count += 1,
                    Some(Payload::CandidacyAnnouncement(_)) => self.candidacy_announcement_count += 1,
                    None => self.no_payload_count += 1,
                }
            }
            BlockBody::Validation(_) => self.validation_count += 1,
        }
        match &block_metadata.block_state {
            Some(state) => match state {
                BlockState::Pending => self.block_pending_count += 1,
                BlockState::Accepted => self.block_accepted_count += 1,
                BlockState::Confirmed => self.block_confirmed_count += 1,
                BlockState::Finalized => self.block_finalized_count += 1,
                BlockState::Dropped => self.block_dropped_count += 1,
                BlockState::Orphaned => self.block_orphaned_count += 1,
            },
            None => self.block_unknown_count += 1,
        }

        Ok(())
    }

    async fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        metadata: &TransactionMetadata,
        _consumed: &[LedgerSpent],
        _created: &[LedgerOutput],
        _ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        match &metadata.transaction_state {
            Some(state) => match state {
                TransactionState::Pending => self.txn_pending_count += 1,
                TransactionState::Accepted => self.txn_accepted_count += 1,
                TransactionState::Committed => self.txn_committed_count += 1,
                TransactionState::Finalized => self.txn_finalized_count += 1,
                TransactionState::Failed => self.txn_failed_count += 1,
            },
            None => (),
        }

        Ok(())
    }

    async fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        Ok(std::mem::take(self))
    }
}
