// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::ops::{AddAssign, SubAssign};

use derive_more::{AddAssign, SubAssign};

use super::TransactionAnalytics;
use crate::{
    db::collections::analytics::LedgerOutputAnalyticsResult,
    types::{
        ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::Output,
    },
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

#[derive(Copy, Clone, Debug, Default)]
pub struct LedgerOutputAnalytics {
    pub alias: CountValue,
    pub basic: CountValue,
    pub nft: CountValue,
    pub foundry: CountValue,
    pub treasury: CountValue,
}

impl LedgerOutputAnalytics {
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

impl TransactionAnalytics for LedgerOutputAnalytics {
    type Measurement = LedgerOutputAnalyticsResult;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        for input in consumed {
            match input.output.output {
                Output::Alias(_) => self.alias -= input,
                Output::Basic(_) => self.basic -= input,
                Output::Nft(_) => self.nft -= input,
                Output::Foundry(_) => self.foundry -= input,
                Output::Treasury(_) => self.treasury -= input,
            }
        }

        for output in created {
            match output.output {
                Output::Alias(_) => self.alias += output,
                Output::Basic(_) => self.basic += output,
                Output::Nft(_) => self.nft += output,
                Output::Foundry(_) => self.foundry += output,
                Output::Treasury(_) => self.treasury += output,
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(LedgerOutputAnalyticsResult {
            basic_count: self.basic.count as _,
            basic_value: self.basic.value,
            alias_count: self.alias.count as _,
            alias_value: self.alias.value,
            foundry_count: self.foundry.count as _,
            foundry_value: self.foundry.value,
            nft_count: self.nft.count as _,
            nft_value: self.nft.value,
            treasury_count: self.treasury.count as _,
            treasury_value: self.treasury.value,
        })
    }
}
