// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, ops::{RangeBounds, Bound}};

use async_trait::async_trait;
use futures::{stream::{BoxStream, self}, StreamExt};
use thiserror::Error;

use super::{BlockData, InputSource, MilestoneData};
use crate::{tangle::ledger_updates::LedgerUpdateStore, types::tangle::MilestoneIndex};

type WhiteFlagIndex = u32;

#[derive(Clone, Debug)]
pub(crate) struct InMemoryTangle {
    milestones: Vec<InMemoryMilestone>,
}

#[derive(Clone, Debug)]
pub(crate) struct InMemoryMilestone {
    pub(crate) milestone: MilestoneData,
    pub(crate) cone: BTreeMap<WhiteFlagIndex, BlockData>
}

#[derive(Debug, Error)]
pub(crate) enum InMemoryError {
    #[error("missing block data for milestone {0}")]
    MissingBlockData(MilestoneIndex),
}

// This can be removed once https://doc.rust-lang.org/stable/std/ops/enum.Bound.html#method.map is stablized.
fn to_usize(bound: Bound<&MilestoneIndex>) -> Bound<usize> {
    match bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(x) => Bound::Included(x.0 as usize),
        Bound::Excluded(x) => Bound::Excluded(x.0 as usize),
    }
}

#[async_trait]
impl InputSource for InMemoryTangle {
    type Error = InMemoryError;

    async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error> {
        let from = to_usize(range.start_bound());
        let end = to_usize(range.end_bound());
        Ok(stream::iter(self.milestones[(from,end)].iter().map(|InMemoryMilestone{milestone, ..}| Ok(milestone.clone()))).boxed())
    }

    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error> {
        todo!()
    }

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error> {
        todo!()
    }
}
