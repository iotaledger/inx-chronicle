// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream};

use super::{cone_stream::ConeStream, sources::InputSource};
use crate::types::{
    ledger::MilestoneIndexTimestamp,
    stardust::block::payload::{MilestoneId, MilestonePayload},
    tangle::ProtocolParameters,
};

pub struct Milestone<'a, I: InputSource> {
    pub(super) source: &'a I,
    pub milestone_id: MilestoneId,
    pub at: MilestoneIndexTimestamp,
    pub payload: MilestonePayload,
    pub protocol_params: ProtocolParameters,
}

impl<'a, I: InputSource> Milestone<'a, I> {
    /// Returns the blocks of a milestone in white-flag order.
    pub async fn cone_stream(&self) -> Result<ConeStream<'a, I>, I::Error> {
        Ok(ConeStream {
            inner: self.source.cone_stream(self.at.milestone_index).await?,
            store: self.source.ledger_updates(self.at.milestone_index),
        })
    }
}

pub struct MilestoneStream<'a, I: InputSource> {
    pub(super) inner: BoxStream<'a, Result<Milestone<'a, I>, I::Error>>,
}

impl<'a, I: InputSource> Stream for MilestoneStream<'a, I> {
    type Item = Result<Milestone<'a, I>, I::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
}
