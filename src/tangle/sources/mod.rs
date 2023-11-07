// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "inx")]
pub(crate) mod inx;
pub(crate) mod memory;
pub(crate) mod mongodb;

use core::ops::RangeBounds;

use async_trait::async_trait;
use futures::stream::BoxStream;
use iota_sdk::types::block::{slot::SlotIndex, BlockId, SignedBlock};

use crate::{
    inx::{
        ledger::LedgerUpdateStore,
        responses::{BlockMetadata, Commitment, NodeConfiguration},
    },
    model::raw::Raw,
};

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct SlotData {
    pub commitment: Commitment,
    pub node_config: NodeConfiguration,
}

/// Logical grouping of data that belongs to a block.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct BlockData {
    pub block_id: BlockId,
    pub block: Raw<SignedBlock>,
    pub metadata: BlockMetadata,
}

/// Defines a type as a source for milestone and cone stream data.
#[async_trait]
pub trait InputSource: Send + Sync {
    /// The error type for this input source.
    type Error: 'static + std::error::Error + std::fmt::Debug + Send + Sync;

    /// A stream of slots and their commitment data.
    async fn slot_stream(
        &self,
        range: impl RangeBounds<SlotIndex> + Send,
    ) -> Result<BoxStream<Result<SlotData, Self::Error>>, Self::Error>;

    /// A stream of confirmed blocks for a given slot index.
    async fn confirmed_blocks(
        &self,
        index: SlotIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error>;

    /// Retrieves the updates to the ledger for a given range of slots.
    async fn ledger_updates(&self, index: SlotIndex) -> Result<LedgerUpdateStore, Self::Error>;
}
