// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{pin::Pin, task::{Context, Poll}};

use futures::{stream::BoxStream, Stream};

use crate::types::{stardust::block::payload::{MilestoneId, MilestonePayload}, ledger::MilestoneIndexTimestamp, tangle::ProtocolParameters};

use super::{sources::InputSource, cone_stream::ConeStream};

pub struct Milestone<'a, I: InputSource> {
    input_source: &'a I,
    pub milestone_id: MilestoneId,
    pub at: MilestoneIndexTimestamp,
    pub payload: MilestonePayload,
    pub protocol_params: ProtocolParameters,
}

impl<'a, I: InputSource> Milestone<'a, I> {
   /// Returns the blocks of a milestone in white-flag order.
    pub fn cone_stream() -> Result<ConeStream<'a, I>, I::Error> {
        todo!();
    }
}

pub struct MilestoneStream<'a, I: InputSource> {
    inner: BoxStream<'a, Result<Milestone<'a, I>, I::Error>>,
}

impl<'a, I: InputSource> Stream for MilestoneStream<'a, I> {
    type Item = Result<Milestone<'a, I>, I::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
}
