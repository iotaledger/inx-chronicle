// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::ops::{AddAssign, SubAssign};

use crate::types::{ledger::{LedgerOutput, LedgerSpent}, stardust::block::Output, tangle::MilestoneIndex};

use super::TransactionAnalytics;

#[derive(Copy, Clone, Debug, Default)]
pub struct CountValue {
    pub count: usize,
    pub value: usize,
}

impl AddAssign<&LedgerOutput> for CountValue {
    fn add_assign(&mut self, rhs: &LedgerOutput) {
        self.count += 1;
        self.value += rhs.output.amount().0 as usize;
    }
}

impl SubAssign<&LedgerSpent> for CountValue {
    fn sub_assign(&mut self, rhs: &LedgerSpent) {
        self.count -= 1;
        self.value -= rhs.output.output.amount().0 as usize;
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct OutputStatistics {
    pub alias: CountValue,
    pub basic: CountValue,
    pub nft: CountValue,
    pub foundry: CountValue,
    pub treasury: CountValue,
}

pub struct OutputState {
    measurement: OutputStatistics,
}

impl OutputState {
    /// Initialize the analytics be reading the current ledger state.
    pub fn init<'a>(unspent_outputs: impl Iterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = OutputStatistics::default();

        for output in unspent_outputs {
            match output.output {
                Output::Alias(_) => measurement.alias += output,
                Output::Basic(_) => measurement.basic += output,
                Output::Nft(_) => measurement.nft += output,
                Output::Foundry(_) => measurement.foundry += output,
                Output::Treasury(_) => measurement.treasury += output,
            }
        }

        Self { measurement }
    }
}

impl TransactionAnalytics for OutputState {
    type Measurement = OutputStatistics;

    fn begin_milestone(&mut self, _: MilestoneIndex) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        
        for input in inputs {
            match input.output.output {
                Output::Alias(_) => self.measurement.alias -= input,
                Output::Basic(_) => self.measurement.basic -= input,
                Output::Nft(_) => self.measurement.nft -= input,
                Output::Foundry(_) => self.measurement.foundry -= input,
                Output::Treasury(_) => self.measurement.treasury -= input,
            }
        }

        for output in outputs {
            match output.output {
                Output::Alias(_) => self.measurement.alias += output,
                Output::Basic(_) => self.measurement.basic += output,
                Output::Nft(_) => self.measurement.nft += output,
                Output::Foundry(_) => self.measurement.foundry += output,
                Output::Treasury(_) => self.measurement.treasury += output,
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement.clone())
    }
}
