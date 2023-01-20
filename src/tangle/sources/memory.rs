// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, ops::RangeBounds};

use async_trait::async_trait;
use futures::stream::BoxStream;
use thiserror::Error;

use super::{BlockData, InputSource, MilestoneData};
use crate::{tangle::ledger_updates::LedgerUpdateStore, types::tangle::MilestoneIndex};

pub struct InMemoryData {
    pub milestone: MilestoneData,
    pub cone: BTreeMap<u32, BlockData>,
    pub ledger_updates: LedgerUpdateStore,
}

#[derive(Debug, Error)]
pub enum InMemoryError {
    #[error("missing block data for milestone {0}")]
    MissingBlockData(MilestoneIndex),
}

#[async_trait]
impl InputSource for BTreeMap<MilestoneIndex, InMemoryData> {
    type Error = InMemoryError;

    async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error> {
        Ok(Box::pin(futures::stream::iter(
            self.range(range).map(|(_, v)| Ok(v.milestone.clone())),
        )))
    }

    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error> {
        let cone = &self.get(&index).ok_or(InMemoryError::MissingBlockData(index))?.cone;
        Ok(Box::pin(futures::stream::iter(cone.values().map(|v| Ok(v.clone())))))
    }

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error> {
        Ok(self
            .get(&index)
            .ok_or(InMemoryError::MissingBlockData(index))?
            .ledger_updates
            .clone())
    }
}
