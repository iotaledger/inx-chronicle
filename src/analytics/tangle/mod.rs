// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the tangle.

pub use self::block_activity::BlockActivityAnalytics;
use crate::types::{ledger::BlockMetadata, stardust::block::Block, tangle::MilestoneIndex};

mod block_activity;

#[allow(missing_docs)]
pub trait BlockAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, index: MilestoneIndex);
    fn handle_block(&mut self, block: &Block, block_metadata: &BlockMetadata);
    fn end_milestone(&mut self, index: MilestoneIndex) -> Option<Self::Measurement>;
}
