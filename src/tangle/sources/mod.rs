// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "inx")]
mod inx;
mod mongodb;

use async_trait::async_trait;
use futures::stream::BoxStream;

use super::{ledger_updates::LedgerUpdateStore, milestone_range::MilestoneRange};
use crate::types::{
    ledger::{BlockMetadata, MilestoneIndexTimestamp},
    stardust::block::{
        payload::{MilestoneId, MilestonePayload},
        Block, BlockId,
    },
    tangle::{MilestoneIndex, ProtocolParameters},
};

/// Logical grouping of data that belongs to a milestone.
pub struct MilestoneData {
    pub milestone_id: MilestoneId,
    pub at: MilestoneIndexTimestamp,
    pub payload: MilestonePayload,
    pub protocol_params: ProtocolParameters,
}

/// Logical grouping of data that belongs to a block.
pub struct BlockData {
    pub block_id: BlockId,
    pub block: Block,
    pub raw: Vec<u8>,
    pub metadata: BlockMetadata,
}

/// Defines a type as a source for milestone and cone stream data.
#[async_trait]
pub trait InputSource {
    /// The error type for this input source.
    type Error: 'static + std::error::Error + std::fmt::Debug;

    /// Retrieves a stream of milestones and their protocol parameters given a range of indexes.
    async fn milestone_stream(
        &self,
        range: MilestoneRange,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error>;

    /// Retrieves a stream of blocks and their metadata in white-flag order given a milestone index.
    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error>;

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error>;
}
