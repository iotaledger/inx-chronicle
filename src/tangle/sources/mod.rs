// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "inx")]
pub(crate) mod inx;
pub(crate) mod memory;
pub(crate) mod mongodb;

use core::ops::RangeBounds;

use async_trait::async_trait;
use futures::stream::BoxStream;
use iota_sdk::types::block::slot::SlotIndex;

use crate::model::{block_metadata::BlockWithMetadata, ledger::LedgerUpdateStore, slot::Commitment};

/// Defines a type as a source for block and ledger update data.
#[async_trait]
pub trait InputSource: Send + Sync {
    /// The error type for this input source.
    type Error: 'static + std::error::Error + std::fmt::Debug + Send + Sync;

    /// A stream of slots and their commitment data.
    async fn commitment_stream(
        &self,
        range: impl RangeBounds<SlotIndex> + Send,
    ) -> Result<BoxStream<Result<Commitment, Self::Error>>, Self::Error>;

    /// A stream of accepted blocks for a given slot index.
    async fn accepted_blocks(
        &self,
        index: SlotIndex,
    ) -> Result<BoxStream<Result<BlockWithMetadata, Self::Error>>, Self::Error>;

    /// Retrieves the updates to the ledger for a given range of slots.
    async fn ledger_updates(&self, index: SlotIndex) -> Result<LedgerUpdateStore, Self::Error>;
}
