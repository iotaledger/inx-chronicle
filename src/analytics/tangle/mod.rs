// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the tangle.

use crate::types::{ledger::BlockMetadata, stardust::block::Block, tangle::MilestoneIndex};

mod block_activity;
mod milestone_size;

pub use self::{
    block_activity::{BlockActivity, BlockActivityAnalytics},
    milestone_size::{MilestoneSizeAnalytics, MilestoneSizeMeasurement},
};

#[allow(missing_docs)]
pub trait BlockAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, index: MilestoneIndex);
    fn handle_block(&mut self, block: &Block, raw_block: &[u8], block_metadata: &BlockMetadata);
    fn end_milestone(&mut self, index: MilestoneIndex) -> Option<Self::Measurement>;
}
