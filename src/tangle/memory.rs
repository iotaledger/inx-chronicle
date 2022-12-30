// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Provides an in-memory representation of the tangle.
//!
//! This representation can be useful for writing test cases, for example.

use std::{collections::BTreeMap, convert::Infallible};

use futures::{
    stream::{self, BoxStream},
    StreamExt,
};

use super::{Backend, Milestones};
use crate::{
    inx::BlockWithMetadataMessage,
    types::{stardust::milestone::MilestoneTimestamp, tangle::MilestoneIndex},
};

#[derive(Clone, Debug)]
pub(crate) struct MilestoneInMemory {
    timestamp: MilestoneTimestamp,
    // TODO: Add timestamp
    blocks: Vec<BlockWithMetadataMessage>,
    // leder_updates: Vec<LedgerUpdateMessage>,
}

impl MilestoneInMemory {
    
}

#[derive(Clone, Debug, Default)]
pub(crate) struct InMemory {
    milestones: Vec<MilestoneInMemory>,
}

impl InMemory {
    pub(crate) fn milestones(&self) -> Milestones<Self> {
        let s = stream::iter(self.milestones.iter().enumerate().map(|(index, ms)| {
            // TODO: Clone is bad bad.
            // TODO: Fix Timestamp.
            Ok(super::Milestone {
                backend: self.clone(),
                index: MilestoneIndex(index as u32),
                timestamp: ms.timestamp,
            })
        }));
        Milestones { inner: s.boxed() }
    }

    // TODO: This is more for tests.
    /// Creates an empty milestone.
    pub(crate) fn insert_empty_milestone(&mut self, timestamp: MilestoneTimestamp) -> (MilestoneIndex, &mut MilestoneInMemory) {
        let index = self.milestones.len();
        self.milestones.push(MilestoneInMemory { timestamp, blocks: Vec::new() });
        // Panic: Safe by definition.
        (MilestoneIndex(index as u32), self.milestones.get_mut(index).unwrap())
    }
}

#[async_trait::async_trait]
impl Backend for InMemory {
    type Error = Infallible;

    async fn blocks(
        &mut self,
        milestone_index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockWithMetadataMessage, Self::Error>>, Self::Error> {
        // TODO: handle this case properly
        let milestone = &self.milestones[milestone_index.0 as usize];
        Ok(stream::iter(milestone.blocks.iter().cloned()).map(Result::Ok).boxed())
    }
}

#[cfg(test)]
mod test {
    use futures::TryStreamExt;

    use super::*;

    #[tokio::test]
    async fn example() -> Result<(), <InMemory as Backend>::Error> {
        let mut tangle = InMemory::default();
        tangle.insert_empty_milestone(MilestoneTimestamp(0));
        tangle.insert_empty_milestone(MilestoneTimestamp(1));
        tangle.insert_empty_milestone(MilestoneTimestamp(2));

        let mut ms = tangle.milestones();

        assert_eq!(ms.try_next().await?.unwrap().index, MilestoneIndex(0));
        assert_eq!(ms.try_next().await?.unwrap().index, MilestoneIndex(1));
        assert_eq!(ms.try_next().await?.unwrap().index, MilestoneIndex(2));
        assert_eq!(ms.try_next().await?.map(|m| m.index), None);

        Ok(())
    }
}
