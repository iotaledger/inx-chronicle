// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
    tangle::MilestoneIndex,
};

/// Information about the claiming process.
#[derive(Copy, Clone, Debug, Default)]
pub struct UnclaimedTokens {
    /// The number of outputs that are still unclaimed.
    pub unclaimed_count: usize,
    /// The remaining number of unclaimed tokens.
    pub unclaimed_value: usize,
}

/// Computes information about the claiming process.
pub struct UnclaimedTokensAnalytics {
    measurement: UnclaimedTokens,
}

impl UnclaimedTokensAnalytics {
    /// Initialize the analytics be reading the current ledger state.
    pub async fn init(unspent_outputs: impl Iterator<Item = &LedgerOutput>) -> Self {
        let mut measurement = UnclaimedTokens::default();
        for output in unspent_outputs {
            if output.booked.milestone_index == MilestoneIndex(0) {
                measurement.unclaimed_count += 1;
                measurement.unclaimed_value += output.amount().0 as usize;
            }
        }
        Self { measurement }
    }
}

impl TransactionAnalytics for UnclaimedTokensAnalytics {
    type Measurement = UnclaimedTokens;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], _: &[LedgerOutput]) {
        for input in inputs {
            if input.output.booked.milestone_index == MilestoneIndex(0) {
                self.measurement.unclaimed_count -= 1;
                self.measurement.unclaimed_value -= input.amount().0 as usize;
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(self.measurement)
    }
}
