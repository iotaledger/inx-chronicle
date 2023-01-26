// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::TransactionAnalytics;
use crate::{
    db::collections::analytics::AddressAnalyticsResult,
    types::{
        ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::{output::OutputAmount, Address},
    },
};

/// Computes the number of addresses the currently hold a balance.
pub struct AddressBalanceAnalytics {
    addresses: HashMap<Address, OutputAmount>,
}

impl AddressBalanceAnalytics {
    /// Initialize the analytics be reading the current ledger state.
    pub fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        Self {
            addresses: unspent_outputs
                .into_iter()
                .fold(Default::default(), |mut addresses, output| {
                    if let Some(&a) = output.owning_address() {
                        *addresses.entry(a).or_default() += output.amount();
                    }
                    addresses
                }),
        }
    }
}

impl TransactionAnalytics for AddressBalanceAnalytics {
    type Measurement = AddressAnalyticsResult;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        for input in inputs {
            if let Some(a) = input.owning_address() {
                // All inputs should be present in `addresses`. If not, we skip it's value.
                if let Some(amount) = self.addresses.get_mut(a) {
                    *amount -= input.amount();
                    if *amount == OutputAmount(0) {
                        self.addresses.remove(a);
                    }
                }
            }
        }

        for output in outputs {
            if let Some(&a) = output.owning_address() {
                // All inputs should be present in `addresses`. If not, we skip it's value.
                *self.addresses.entry(a).or_default() += output.amount();
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(AddressAnalyticsResult {
            address_with_balance_count: self.addresses.len() as _,
        })
    }
}
