// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    ledger::BlockMetadata,
    stardust::block::{Block, Output},
    tangle::MilestoneIndex,
};

pub mod address_balance;
pub mod block_activity;
pub mod ledger_size;

#[allow(missing_docs)]
pub trait BlockAnalytics {
    type Measurement;
    type Context;
    fn begin_milestone(&mut self, ctx: Self::Context);
    fn handle_block(&mut self, block: &Block, block_metadata: &BlockMetadata, inputs: &Option<Vec<Output>>);
    fn end_milestone(&mut self, index: MilestoneIndex) -> Option<Self::Measurement>;
}

#[allow(missing_docs)]
pub trait TransactionAnalytics {
    type Measurement;
    type Context;
    fn begin_milestone(&mut self, ctx: Self::Context);
    fn handle_transaction(&mut self, inputs: &[Output], outputs: &[Output]);
    fn end_milestone(&mut self, index: MilestoneIndex) -> Option<Self::Measurement>;
}
