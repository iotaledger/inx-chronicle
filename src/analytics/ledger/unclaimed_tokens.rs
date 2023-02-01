// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
    tangle::MilestoneIndex,
};

/// Information about the claiming process.
#[derive(Copy, Clone, Debug, Default)]
pub struct UnclaimedTokenMeasurement {
    /// The number of outputs that are still unclaimed.
    pub unclaimed_count: usize,
    /// The remaining number of unclaimed tokens.
    pub unclaimed_value: u64,
}

impl UnclaimedTokenMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            if output.booked.milestone_index == MilestoneIndex(0) {
                measurement.unclaimed_count += 1;
                measurement.unclaimed_value += output.amount().0;
            }
        }
        measurement
    }
}

impl TransactionAnalytics for UnclaimedTokenMeasurement {
    type Measurement = Self;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], _: &[LedgerOutput]) {
        for input in inputs {
            if input.output.booked.milestone_index == MilestoneIndex(0) {
                self.unclaimed_count -= 1;
                self.unclaimed_value -= input.amount().0;
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(*self)
    }
}
