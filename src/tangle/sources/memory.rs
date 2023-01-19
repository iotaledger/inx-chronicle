// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, ops::RangeBounds};

use async_trait::async_trait;
use futures::{stream::{BoxStream, self}, StreamExt};
use thiserror::Error;

use super::{BlockData, InputSource, MilestoneData, UnspentOutputData};
use crate::{tangle::ledger_updates::LedgerUpdateStore, types::tangle::MilestoneIndex};

type WhiteFlagIndex = u32;

#[derive(Clone, Debug)]
pub struct InMemoryTangle {
    milestones: Vec<InMemoryMilestone>,
}

#[derive(Clone, Debug)]
pub struct InMemoryMilestone {
    pub milestone: MilestoneData,
    pub cone: BTreeMap<WhiteFlagIndex, BlockData>
}

#[derive(Debug, Error)]
pub enum InMemoryError {
    #[error("missing block data for milestone {0}")]
    MissingBlockData(MilestoneIndex),
}

#[async_trait]
impl InputSource for InMemoryTangle {
    type Error = InMemoryError;

    async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error> {
        Ok(stream::iter(self.milestones.iter().map(|InMemoryMilestone{milestone, ..}| Ok(milestone.clone()))).boxed())
    }

    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error> {
        todo!()
    }

    async fn unspent_outputs(&self) -> Result<BoxStream<Result<UnspentOutputData, Self::Error>>, Self::Error> {
        todo!()
    }

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error> {
        todo!()
    }
}
