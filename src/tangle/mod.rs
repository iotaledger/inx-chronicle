// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Provides an abstraction over the events in the Tangle.

mod inx;
mod memory;

use std::{
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream};

use crate::{
    inx::BlockWithMetadataMessage,
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

#[async_trait::async_trait]
/// A trait for types that model the tangle.
pub trait Backend: Clone {
    /// The error returned when retrieving information about the tangle from the backend.
    type Error: Debug;

    /// Returns the cone of blocks contained in this milestone in white-flag order.
    async fn blocks(
        &mut self,
        milestone_index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockWithMetadataMessage, Self::Error>>, Self::Error>;
}

pub struct Milestones<'a, B: Backend> {
    inner: BoxStream<'a, Result<Milestone<B>, B::Error>>,
}

impl<'a, B: Backend> Stream for Milestones<'a, B> {
    type Item = Result<Milestone<B>, B::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
}

/// Represents a single milestone in the tangle.
pub struct Milestone<B: Backend> {
    // TODO: can this be a `&mut`? That way we wouldn't have to clone the backend.
    backend: B,
    // pub protocol_parameters: ProtocolParameters,
    /// The index of the milestone.
    pub index: MilestoneIndex,
    /// the timestamp of the the milestone.
    pub timestamp: MilestoneTimestamp,
}

impl<B: Backend> Milestone<B> {
    /// Returns the cone of blocks contained in this milestone in white-flag order.
    async fn blocks(&mut self) -> BoxStream<Result<BlockWithMetadataMessage, B::Error>> {
        // Panic: Milestone has to exists by definition.
        self.backend.blocks(self.index).await.unwrap()
    }
}
