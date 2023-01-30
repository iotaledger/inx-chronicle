// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::BlockAnalytics;
use crate::{
    db::collections::analytics::{
        BlockActivityAnalyticsResult, PayloadActivityAnalyticsResult, TransactionActivityAnalyticsResult,
    },
    tangle::BlockData,
    types::{
        ledger::{BlockMetadata, LedgerInclusionState},
        stardust::block::{Block, Payload},
        tangle::MilestoneIndex,
    },
};

/// The type of payloads that occured within a single milestone.
#[derive(Clone, Debug, Default)]
pub struct BlockActivityAnalytics {
    milestone_count: usize,
    no_payload_count: usize,
    tagged_data_count: usize,
    transaction_count: usize,
    treasury_transaction_count: usize,
    confirmed_count: usize,
    conflicting_count: usize,
    no_transaction_count: usize,
}

impl BlockAnalytics for BlockActivityAnalytics {
    type Measurement = BlockActivityAnalyticsResult;

    fn begin_milestone(&mut self, _: MilestoneIndex) {
        *self = Default::default();
    }

    fn handle_block(&mut self, BlockData { block, metadata, .. }: &BlockData) {
        match block.payload {
            Some(Payload::Milestone(_)) => self.milestone_count += 1,
            Some(Payload::TaggedData(_)) => self.tagged_data_count += 1,
            Some(Payload::Transaction(_)) => self.transaction_count += 1,
            Some(Payload::TreasuryTransaction(_)) => self.treasury_transaction_count += 1,
            None => self.no_payload_count += 1,
        }
        match metadata.inclusion_state {
            LedgerInclusionState::Conflicting => self.conflicting_count += 1,
            LedgerInclusionState::Included => self.confirmed_count += 1,
            LedgerInclusionState::NoTransaction => self.no_transaction_count += 1,
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(BlockActivityAnalyticsResult {
            payload: PayloadActivityAnalyticsResult {
                transaction_count: self.transaction_count as _,
                treasury_transaction_count: self.treasury_transaction_count as _,
                milestone_count: self.milestone_count as _,
                tagged_data_count: self.tagged_data_count as _,
                no_payload_count: self.no_payload_count as _,
            },
            transaction: TransactionActivityAnalyticsResult {
                confirmed_count: self.confirmed_count as _,
                conflicting_count: self.conflicting_count as _,
                no_transaction_count: self.no_transaction_count as _,
            },
        })
    }
}
