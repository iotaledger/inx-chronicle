// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::ops::RangeBounds;
use std::collections::BTreeMap;

use async_trait::async_trait;
use futures::stream::BoxStream;
use iota_sdk::types::block::{slot::SlotIndex, BlockId};
use thiserror::Error;

use super::{BlockData, InputSource, SlotData};
use crate::inx::ledger::LedgerUpdateStore;

pub struct InMemoryData {
    pub slot_data: SlotData,
    pub confirmed_blocks: BTreeMap<BlockId, BlockData>,
    pub ledger_updates: LedgerUpdateStore,
}

#[derive(Debug, Error)]
pub enum InMemoryInputSourceError {
    #[error("missing block data for slot {0}")]
    MissingBlockData(SlotIndex),
}

#[async_trait]
impl InputSource for BTreeMap<SlotIndex, InMemoryData> {
    type Error = InMemoryInputSourceError;

    async fn slot_stream(
        &self,
        range: impl RangeBounds<SlotIndex> + Send,
    ) -> Result<BoxStream<Result<SlotData, Self::Error>>, Self::Error> {
        Ok(Box::pin(futures::stream::iter(
            self.range(range).map(|(_, v)| Ok(v.slot_data.clone())),
        )))
    }

    async fn confirmed_blocks(
        &self,
        index: SlotIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error> {
        let blocks = &self
            .get(&index)
            .ok_or(InMemoryInputSourceError::MissingBlockData(index))?
            .confirmed_blocks;
        Ok(Box::pin(futures::stream::iter(blocks.values().map(|v| Ok(v.clone())))))
    }

    async fn ledger_updates(&self, index: SlotIndex) -> Result<LedgerUpdateStore, Self::Error> {
        Ok(self
            .get(&index)
            .ok_or(InMemoryInputSourceError::MissingBlockData(index))?
            .ledger_updates
            .clone())
    }
}
