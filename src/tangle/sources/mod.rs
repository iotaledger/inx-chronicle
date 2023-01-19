// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "inx")]
mod inx;
pub(crate) mod memory;
mod mongodb;

use std::ops::RangeBounds;

use async_trait::async_trait;
use futures::stream::BoxStream;

use super::ledger_updates::LedgerUpdateStore;
use crate::types::{
    ledger::{BlockMetadata, LedgerOutput, MilestoneIndexTimestamp},
    stardust::block::{
        payload::{MilestoneId, MilestonePayload},
        Block, BlockId,
    },
    tangle::{MilestoneIndex, ProtocolParameters},
};

/// Logical grouping of data that belongs to a milestone.
#[derive(Clone, Debug)]
pub struct MilestoneData {
    pub milestone_id: MilestoneId,
    pub at: MilestoneIndexTimestamp,
    pub payload: MilestonePayload,
    pub protocol_params: ProtocolParameters,
}

/// Logical grouping of data that belongs to a block.
#[derive(Clone, Debug)]
pub struct BlockData {
    pub block_id: BlockId,
    pub block: Block,
    pub raw: Vec<u8>,
    pub metadata: BlockMetadata,
}

/// Defines a type as a source for milestone and cone stream data.
#[async_trait]
pub trait InputSource
where
    Self: Send,
{
    /// The error type for this input source.
    type Error: 'static + std::error::Error + std::fmt::Debug;

    /// Retrieves a stream of milestones and their protocol parameters given a range of indexes.
    async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error>;

    /// Retrieves a stream of blocks and their metadata in white-flag order given a milestone index.
    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error>;

    /// Retrieves the updates to the ledger for a given milestone.
    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error>;
}
