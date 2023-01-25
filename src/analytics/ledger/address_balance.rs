// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent},
    stardust::block::Address,
    tangle::MilestoneIndex,
};

/// The number of addresses.
pub struct AddressCount(usize);

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
                addresses.insert(a.clone());
            }
        }
        Self { addresses }
    }
}

impl TransactionAnalytics for AddressBalanceAnalytics {
    type Measurement = AddressCount;

    fn begin_milestone(&mut self, _: MilestoneIndex) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        for input in inputs {
            if let Some(a) = input.output.output.owning_address() {
                self.addresses.remove(a);
            }
        }

        for output in outputs {
            if let Some(a) = output.output.owning_address() {
                self.addresses.insert(a.clone());
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(AddressCount(self.addresses.len()))
    }
}
