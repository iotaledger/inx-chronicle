// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Provides an abstraction over the events in the Tangle.

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream};

use crate::types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex};

/// A trait for types that model the tangle.
pub trait Backend: Clone {
    /// The error returned when retrieving information about the tangle from the backend.
    type Error;
}

struct Milestones<'a, B: Backend> {
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
    backend: B,
    // pub protocol_parameters: ProtocolParameters,
    /// The index of the milestone.
    pub index: MilestoneIndex,
    /// the timestamp of the the milestone.
    pub timestamp: MilestoneTimestamp,
}

#[cfg(test)]
mod test {
    use futures::{stream, StreamExt, TryStreamExt};

    use super::*;

    // Dummy implementation used for testing.
    #[derive(Debug, Clone)]
    struct DummyBackend;

    impl Backend for DummyBackend {
        type Error = ();
    }

    #[tokio::test]
    async fn example() -> Result<(), <DummyBackend as Backend>::Error> {
        let milestones = stream::iter((0u32..=1).map(|i| {
            Ok(Milestone {
                backend: DummyBackend,
                index: i.into(),
                timestamp: i.into(),
            })
        }));
        let mut stream = Milestones {
            inner: milestones.boxed(),
        };

        assert_eq!(stream.try_next().await?.unwrap().index, MilestoneIndex(0));
        assert_eq!(stream.try_next().await?.unwrap().index, MilestoneIndex(1));
        assert_eq!(stream.try_next().await?.map(|m| m.index), None);

        Ok(())
    }
}
