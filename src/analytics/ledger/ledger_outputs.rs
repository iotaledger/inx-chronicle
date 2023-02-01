// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::ops::{AddAssign, SubAssign};

use derive_more::{AddAssign, SubAssign};

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
    stardust::block::Output,
};

#[derive(Copy, Clone, Debug, Default, AddAssign, SubAssign)]
pub struct CountValue {
    pub count: usize,
    pub value: u64,
}

impl AddAssign<&LedgerOutput> for CountValue {
    fn add_assign(&mut self, rhs: &LedgerOutput) {
        self.count += 1;
        self.value += rhs.output.amount().0;
    }
}

impl SubAssign<&LedgerSpent> for CountValue {
    fn sub_assign(&mut self, rhs: &LedgerSpent) {
        self.count -= 1;
        self.value -= rhs.output.output.amount().0;
    }
}

#[derive(Copy, Clone, Debug, Default, AddAssign, SubAssign)]
pub struct LedgerOutputMeasurement {
    pub alias: CountValue,
    pub basic: CountValue,
    pub nft: CountValue,
    pub foundry: CountValue,
    pub treasury: CountValue,
}

impl LedgerOutputMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
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

impl TransactionAnalytics for LedgerOutputMeasurement {
    type Measurement = Self;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        *self += created;
        *self -= consumed;
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(*self)
    }
}
