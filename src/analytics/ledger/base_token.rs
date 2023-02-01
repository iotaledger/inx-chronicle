// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
    stardust::block::Address,
};

/// Measures activity of the base token, such as Shimmer or IOTA.
#[derive(Copy, Clone, Debug, Default)]
pub struct BaseTokenActivityMeasurement {
    /// Represents the amount of tokens transfered. Tokens that are send back to an address are not counted.
    pub booked_value: u64,
    /// Represents the total amount of tokens transfered, independent of wether tokens were sent back to same address.
    pub transferred_value: u64,
}

impl TransactionAnalytics for BaseTokenActivityMeasurement {
    type Measurement = Self;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {
        *self = Default::default();
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        // The idea behind the following code is that we keep track of the deltas that are applied to each account that
        // is represented by an address.
        let mut balance_deltas: HashMap<&Address, u64> = HashMap::new();

        // We first gather all tokens that have been moved to an individual address.
        for output in created {
            if let Some(address) = output.owning_address() {
                *balance_deltas.entry(address).or_default() += output.amount().0;
            }
        }

        self.booked_value = balance_deltas.values().sum();

        // Afterwards, we subtract the tokens from that address to get the actual deltas of each account.
        for input in consumed {
            if let Some(address) = input.owning_address() {
                *balance_deltas.entry(address).or_default() -= input.amount().0;
            }
        }

        // The number of transferred tokens is then the sum of all deltas.
        self.transferred_value = balance_deltas.values().sum();
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(*self)
    }
}
