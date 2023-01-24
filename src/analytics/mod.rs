// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    ledger::{BlockMetadata, LedgerOutput, LedgerSpent},
    stardust::block::{Block, Output},
    tangle::MilestoneIndex,
};

pub mod address_balance;
pub mod base_token;
pub mod block_activity;
pub mod unclaimed_tokens;

#[allow(missing_docs)]
pub trait BlockAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, index: MilestoneIndex);
    fn handle_block(&mut self, block: &Block, block_metadata: &BlockMetadata, inputs: &Option<Vec<Output>>);
    fn end_milestone(&mut self, index: MilestoneIndex) -> Option<Self::Measurement>;
}

#[allow(missing_docs)]
pub trait TransactionAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, index: MilestoneIndex);
    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]);
    fn end_milestone(&mut self, index: MilestoneIndex) -> Option<Self::Measurement>;
}

// pub fn analytics(milestones: impl Stream<Item=Milestone>)
