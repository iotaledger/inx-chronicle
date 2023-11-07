// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use iota_sdk::types::block::output::Output;

use super::*;

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct LedgerOutputMeasurement {
    pub(crate) account: CountAndAmount,
    pub(crate) basic: CountAndAmount,
    pub(crate) nft: CountAndAmount,
    pub(crate) foundry: CountAndAmount,
    pub(crate) anchor: CountAndAmount,
    pub(crate) delegation: CountAndAmount,
}

impl LedgerOutputMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            match output.output {
                Output::Account(_) => measurement.account.add_output(output),
                Output::Basic(_) => measurement.basic.add_output(output),
                Output::Nft(_) => measurement.nft.add_output(output),
                Output::Foundry(_) => measurement.foundry.add_output(output),
                Output::Anchor(_) => measurement.anchor.add_output(output),
                Output::Delegation(_) => measurement.delegation.add_output(output),
            }
        }
        measurement
    }

    fn wrapping_add(&mut self, rhs: Self) {
        self.account.wrapping_add(rhs.account);
        self.basic.wrapping_add(rhs.basic);
        self.nft.wrapping_add(rhs.nft);
        self.foundry.wrapping_add(rhs.foundry);
        self.anchor.wrapping_add(rhs.anchor);
        self.delegation.wrapping_add(rhs.delegation);
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        self.account.wrapping_sub(rhs.account);
        self.basic.wrapping_sub(rhs.basic);
        self.nft.wrapping_sub(rhs.nft);
        self.foundry.wrapping_sub(rhs.foundry);
        self.anchor.wrapping_sub(rhs.anchor);
        self.delegation.wrapping_sub(rhs.delegation);
    }
}

impl Analytics for LedgerOutputMeasurement {
    type Measurement = Self;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        self.wrapping_sub(consumed);
        self.wrapping_add(created);
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        *self
    }
}
