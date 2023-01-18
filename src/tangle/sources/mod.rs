// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "inx")]
mod inx;
mod mongodb;

use async_trait::async_trait;
use futures::stream::BoxStream;

use super::{cone_stream::BlockWithMetadataInputs, ledger_updates::LedgerUpdateStore};
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

#[derive(Copy, Clone, Debug)]
pub struct MilestoneRange {
    pub start: MilestoneIndex,
    pub end: MilestoneIndex,
}

impl<T> From<T> for MilestoneRange
where
    T: std::ops::RangeBounds<MilestoneIndex>,
{
    fn from(value: T) -> MilestoneRange {
        use std::ops::Bound;
        let start = match value.start_bound() {
            Bound::Included(&idx) => idx,
            Bound::Excluded(&idx) => idx + 1,
            Bound::Unbounded => 0.into(),
        };
        let end = match value.end_bound() {
            Bound::Included(&idx) => idx,
            Bound::Excluded(&idx) => idx - 1,
            Bound::Unbounded => u32::MAX.into(),
        };
        MilestoneRange { start, end }
    }
}

impl Iterator for MilestoneRange {
    type Item = MilestoneIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let res = self.start;
            self.start += 1;
            Some(res)
        } else {
            None
        }
    }
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
    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockWithMetadataInputs, Self::Error>>, Self::Error>;

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error>;
}
