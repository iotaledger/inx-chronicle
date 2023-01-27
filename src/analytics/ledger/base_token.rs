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
        // The idea behind the following code is that we keep track of the deltas that are applied to each account that is represented by an address.
        let mut balance_deltas: HashMap<&Address, usize> = HashMap::new();

        // We first gather all tokens that have been moved to an individual address.
        for output in outputs {
            if let Some(address) = output.owning_address() {
                *balance_deltas.entry(address).or_default() += output.amount().0 as usize;
            }
        }

        self.measurement.booked_value = balance_deltas.values().sum();

        // Afterwards, we subtract the tokens from that address to get the actual deltas of each account.
        for input in inputs {
            if let Some(address) = input.owning_address() {
                *balance_deltas.entry(address).or_default() -= input.amount().0 as usize;
            }
        }

        // The number of transferred tokens is then the sum of all deltas.
        self.measurement.transferred_value = balance_deltas.values().sum();
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(self.measurement.clone())
    }
}
