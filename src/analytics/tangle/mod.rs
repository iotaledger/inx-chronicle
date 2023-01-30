// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the tangle.

pub use self::block_activity::BlockActivityAnalytics;
use crate::{tangle::BlockData, types::tangle::MilestoneIndex};

mod block_activity;
mod milestone_size;

pub use self::milestone_size::{MilestoneSizeAnalytics, MilestoneSizeMeasurement};

#[allow(missing_docs)]
pub trait BlockAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, index: MilestoneIndex);
    fn handle_block(&mut self, block: &BlockData);
    fn end_milestone(&mut self, index: MilestoneIndex) -> Option<Self::Measurement>;
}
