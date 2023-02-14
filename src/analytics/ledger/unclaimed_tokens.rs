// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

/// Information about the claiming process.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct UnclaimedTokenMeasurement {
    /// The number of outputs that are still unclaimed.
    pub(crate) unclaimed_count: usize,
    /// The remaining number of unclaimed tokens.
    pub(crate) unclaimed_value: TokenAmount,
}

impl UnclaimedTokenMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            if output.booked.milestone_index == 0 {
                measurement.unclaimed_count += 1;
                measurement.unclaimed_value += output.amount();
            }
        }
        measurement
    }
}

impl Analytics for UnclaimedTokenMeasurement {
    type Measurement = Self;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], _: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        for input in inputs {
            if input.output.booked.milestone_index == 0 {
                self.unclaimed_count -= 1;
                self.unclaimed_value -= input.amount();
            }
        }
    }

    fn end_milestone(&mut self, _ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        Some(*self)
    }
}
