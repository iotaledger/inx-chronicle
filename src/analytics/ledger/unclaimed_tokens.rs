// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::TransactionAnalytics;
use crate::{
    db::collections::analytics::UnclaimedTokenAnalyticsResult,
    types::{
        ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        tangle::MilestoneIndex,
    },
};

/// Information about the claiming process.
#[derive(Copy, Clone, Debug, Default)]
pub struct UnclaimedTokenAnalytics {
    /// The number of outputs that are still unclaimed.
    pub unclaimed_count: usize,
    /// The remaining number of unclaimed tokens.
    pub unclaimed_value: usize,
}

impl UnclaimedTokenAnalytics {
    /// Initialize the analytics be reading the current ledger state.
    pub fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        unspent_outputs
            .into_iter()
            .fold(Self::default(), |mut measurement, output| {
                if output.booked.milestone_index == MilestoneIndex(0) {
                    measurement.unclaimed_count += 1;
                    measurement.unclaimed_value += output.amount().0 as usize;
                }
                measurement
            })
    }
}

impl TransactionAnalytics for UnclaimedTokenAnalytics {
    type Measurement = UnclaimedTokenAnalyticsResult;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], _: &[LedgerOutput]) {
        for input in inputs {
            if input.output.booked.milestone_index == MilestoneIndex(0) {
                self.unclaimed_count -= 1;
                self.unclaimed_value -= input.amount().0 as usize;
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(UnclaimedTokenAnalyticsResult {
            unclaimed_count: self.unclaimed_count as _,
            unclaimed_value: self.unclaimed_value as _,
        })
    }
}
