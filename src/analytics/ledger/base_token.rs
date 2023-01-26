// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
    stardust::block::Address,
};

/// Measures activity of the base token, such as Shimmer or IOTA.
#[derive(Clone, Debug, Default)]
pub struct BaseTokenActivity {
    /// Represents the amount of tokens transfered. Tokens that are send back to an address are not counted.
    pub booked_value: usize,
    /// Represents the total amount of tokens transfered, independent of wether tokens were sent back to same address.
    pub transferred_value: usize,
}

/// Computes information about the usage of the underlying base.
pub struct BaseTokenActivityAnalytics {
    measurement: BaseTokenActivity,
}

impl TransactionAnalytics for BaseTokenActivityAnalytics {
    type Measurement = BaseTokenActivity;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {
        self.measurement = BaseTokenActivity::default();
    }

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        let mut tmp_balances: HashMap<&Address, usize> = HashMap::new();

        for output in outputs {
            if let Some(address) = output.owning_address() {
                *tmp_balances.entry(address).or_default() += output.amount().0 as usize;
            }
        }

        self.measurement.booked_value = tmp_balances.values().sum();

        for input in inputs {
            if let Some(address) = input.owning_address() {
                *tmp_balances.entry(address).or_default() -= input.amount().0 as usize;
            }
        }

        self.measurement.transferred_value = tmp_balances.values().sum();
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(self.measurement.clone())
    }
}