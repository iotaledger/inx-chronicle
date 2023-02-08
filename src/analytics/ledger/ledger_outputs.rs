// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use super::*;

#[derive(Copy, Clone, Debug, Default, AddAssign, SubAssign)]
pub(crate) struct LedgerOutputMeasurement {
    pub(crate) alias: CountAndAmount,
    pub(crate) basic: CountAndAmount,
    pub(crate) nft: CountAndAmount,
    pub(crate) foundry: CountAndAmount,
    pub(crate) treasury: CountAndAmount,
}

impl LedgerOutputMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            match output.output {
                Output::Alias(_) => measurement.alias += output,
                Output::Basic(_) => measurement.basic += output,
                Output::Nft(_) => measurement.nft += output,
                Output::Foundry(_) => measurement.foundry += output,
                Output::Treasury(_) => measurement.treasury += output,
            }
        }
        measurement
    }
}

impl Analytics for LedgerOutputMeasurement {
    type Measurement = PerMilestone<Self>;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        *self -= consumed;
        *self += created;
    }

    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        Some(PerMilestone {
            at: *ctx.at(),
            inner: *self,
        })
    }
}
