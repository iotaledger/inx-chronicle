// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "inx")]
mod inx;
mod mongodb;

use async_trait::async_trait;
use futures::stream::BoxStream;

use super::{cone_stream::BlockWithMetadataInputs, ledger_updates::LedgerUpdateStore, milestone_range::MilestoneRange};
use crate::types::{
    ledger::MilestoneIndexTimestamp,
    stardust::block::payload::{MilestoneId, MilestonePayload},
    tangle::{MilestoneIndex, ProtocolParameters},
};

#[allow(missing_docs)]
pub struct MilestoneAndProtocolParameters {
    pub milestone_id: MilestoneId,
    pub at: MilestoneIndexTimestamp,
    pub payload: MilestonePayload,
    pub protocol_params: ProtocolParameters,
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
    ) -> Result<BoxStream<Result<MilestoneAndProtocolParameters, Self::Error>>, Self::Error>;

    /// Retrieves a stream of blocks and their metadata in white-flag order given a milestone index.
    // TODO: This should not require enriching the inputs already
    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockWithMetadataInputs, Self::Error>>, Self::Error>;

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error>;
}
