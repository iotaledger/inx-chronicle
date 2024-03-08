// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::ops::RangeBounds;
use std::collections::BTreeMap;

use async_trait::async_trait;
use futures::stream::BoxStream;
use iota_sdk::types::block::{payload::signed_transaction::TransactionId, slot::SlotIndex, BlockId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::InputSource;
use crate::model::{
    block_metadata::{BlockWithMetadata, TransactionMetadata},
    ledger::LedgerUpdateStore,
    slot::Commitment,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InMemoryData {
    pub commitment: Commitment,
    pub committed_blocks: BTreeMap<BlockId, BlockWithMetadata>,
    pub transaction_metadata: BTreeMap<TransactionId, TransactionMetadata>,
    pub ledger_updates: LedgerUpdateStore,
}

#[derive(Debug, Error)]
pub enum InMemoryInputSourceError {
    #[error("missing block data for slot {0}")]
    MissingBlockData(SlotIndex),
    #[error("missing metadata for transaction {0}")]
    MissingTransactionMetadata(TransactionId),
}

#[async_trait]
impl InputSource for BTreeMap<SlotIndex, InMemoryData> {
    type Error = InMemoryInputSourceError;

    async fn commitment_stream(
        &self,
        range: impl RangeBounds<SlotIndex> + Send,
    ) -> Result<BoxStream<Result<Commitment, Self::Error>>, Self::Error> {
        Ok(Box::pin(futures::stream::iter(
            self.range(range).map(|(_, v)| Ok(v.commitment.clone())),
        )))
    }

    async fn accepted_blocks(
        &self,
        index: SlotIndex,
    ) -> Result<BoxStream<Result<BlockWithMetadata, Self::Error>>, Self::Error> {
        let blocks = &self
            .get(&index)
            .ok_or(InMemoryInputSourceError::MissingBlockData(index))?
            .committed_blocks;
        Ok(Box::pin(futures::stream::iter(blocks.values().map(|v| Ok(v.clone())))))
    }

    async fn transaction_metadata(&self, transaction_id: TransactionId) -> Result<TransactionMetadata, Self::Error> {
        let index = transaction_id.slot_index();
        Ok(self
            .get(&index)
            .ok_or(InMemoryInputSourceError::MissingBlockData(index))?
            .transaction_metadata
            .get(&transaction_id)
            .ok_or(InMemoryInputSourceError::MissingTransactionMetadata(transaction_id))?
            .clone())
    }

    async fn ledger_updates(&self, index: SlotIndex) -> Result<LedgerUpdateStore, Self::Error> {
        Ok(self
            .get(&index)
            .ok_or(InMemoryInputSourceError::MissingBlockData(index))?
            .ledger_updates
            .clone())
    }
}
