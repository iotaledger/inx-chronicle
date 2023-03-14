// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use influxdb::WriteQuery;

use super::*;
use crate::analytics::measurement::Measurement;

/// Information about the claiming process.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct UnclaimedTokenMeasurement {
    /// The number of outputs that are still unclaimed.
    pub(crate) unclaimed_count: usize,
    /// The remaining number of unclaimed tokens.
    pub(crate) unclaimed_amount: TokenAmount,
}

impl UnclaimedTokenMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            if output.booked.milestone_index == 0 {
                measurement.unclaimed_count += 1;
                measurement.unclaimed_amount += output.amount();
            }
        }
        measurement
    }
}

impl Analytics for UnclaimedTokenMeasurement {
    type Measurement = Self;

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], _: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        for input in inputs {
            if input.output.booked.milestone_index == 0 {
                self.unclaimed_count -= 1;
                self.unclaimed_amount -= input.amount();
            }
        }
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        *self
    }
}

impl Measurement for UnclaimedTokenMeasurement {
    const NAME: &'static str = "stardust_unclaimed_rewards";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("unclaimed_count", self.unclaimed_count as u64)
            .add_field("unclaimed_amount", self.unclaimed_amount.0)
    }
}
