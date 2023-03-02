// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream};

use super::{
    sources::{BlockData, InputSource},
    LedgerUpdateStore,
};
use crate::model::{MilestoneId, MilestoneIndexTimestamp, MilestonePayload, NodeConfiguration, ProtocolParameters};

#[allow(missing_docs)]
pub struct Milestone<'a, I: InputSource> {
    pub(super) source: &'a I,
    pub milestone_id: MilestoneId,
    pub at: MilestoneIndexTimestamp,
    pub payload: MilestonePayload,
    pub protocol_params: ProtocolParameters,
    pub node_config: NodeConfiguration,
    pub ledger_updates: LedgerUpdateStore,
}

impl<'a, I: InputSource> Milestone<'a, I> {
    /// Returns the blocks of a milestone in white-flag order.
    pub async fn cone_stream(&self) -> Result<BoxStream<Result<BlockData, I::Error>>, I::Error> {
        self.source.cone_stream(self.at.milestone_index).await
    }

    /// Returns the ledger update store.
    pub fn ledger_updates(&self) -> &LedgerUpdateStore {
        &self.ledger_updates
    }
}

#[allow(missing_docs)]
pub struct MilestoneStream<'a, I: InputSource> {
    pub(super) inner: BoxStream<'a, Result<Milestone<'a, I>, I::Error>>,
}

impl<'a, I: InputSource> Stream for MilestoneStream<'a, I> {
    type Item = Result<Milestone<'a, I>, I::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
}
