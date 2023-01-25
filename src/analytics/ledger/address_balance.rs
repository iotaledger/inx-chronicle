// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use super::{AddressCount, TransactionAnalytics};
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
    stardust::block::Address,
};

/// Computes the number of addresses the currently hold a balance.
pub struct AddressBalanceAnalytics {
    addresses: HashSet<Address>,
}

impl AddressBalanceAnalytics {
    /// Initialize the analytics be reading the current ledger state.
    pub fn init<'a>(unspent_outputs: impl Iterator<Item = &'a LedgerOutput>) -> Self {
        let mut addresses = HashSet::new();
        for output in unspent_outputs {
            if let Some(a) = output.output.owning_address() {
                addresses.insert(*a);
            }
        }
        Self { addresses }
    }
}

impl TransactionAnalytics for AddressBalanceAnalytics {
    type Measurement = AddressCount;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        for input in inputs {
            if let Some(a) = input.output.output.owning_address() {
                self.addresses.remove(a);
            }
        }

        for output in outputs {
            if let Some(a) = output.output.owning_address() {
                self.addresses.insert(*a);
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(AddressCount(self.addresses.len()))
    }
}
