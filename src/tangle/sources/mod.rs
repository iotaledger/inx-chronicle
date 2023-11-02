// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "inx")]
pub(crate) mod inx;
pub(crate) mod memory;
pub(crate) mod mongodb;
use std::ops::RangeBounds;

use async_trait::async_trait;
use futures::stream::BoxStream;
use iota_sdk::types::{
    api::core::BlockMetadataResponse,
    block::{slot::SlotIndex, BlockDto, BlockId},
};

use super::ledger_updates::LedgerUpdateStore;

/// Logical grouping of data that belongs to a block.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct BlockData {
    pub block_id: BlockId,
    pub block: BlockDto,
    pub raw: Vec<u8>,
    pub metadata: BlockMetadataResponse,
}

/// Defines a type as a source for milestone and cone stream data.
#[async_trait]
pub trait InputSource: Send + Sync {
    /// The error type for this input source.
    type Error: 'static + std::error::Error + std::fmt::Debug + Send + Sync;

    /// Retrieves the updates to the ledger for a given range of slots.
    async fn ledger_updates(&self, range: impl RangeBounds<SlotIndex> + Send)
    -> Result<LedgerUpdateStore, Self::Error>;
}
