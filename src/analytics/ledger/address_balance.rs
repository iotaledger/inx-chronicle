// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::*;
use crate::types::stardust::block::{output::OutputAmount, Address};

pub(crate) struct AddressBalanceMeasurement {
    pub(crate) address_with_balance_count: usize,
}

/// Computes the number of addresses the currently hold a balance.
pub(crate) struct AddressBalancesAnalytics {
    balances: HashMap<Address, OutputAmount>,
}

impl AddressBalancesAnalytics {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut balances = HashMap::new();
        for output in unspent_outputs {
            if let Some(&a) = output.owning_address() {
                *balances.entry(a).or_default() += output.amount();
            }
        }
        Self { balances }
    }
}

impl Analytics for AddressBalancesAnalytics {
    type Measurement = PerMilestone<AddressBalanceMeasurement>;

    fn begin_milestone(&mut self, _at: MilestoneIndexTimestamp, _params: &ProtocolParameters) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        for input in consumed {
            if let Some(a) = input.owning_address() {
                // All inputs should be present in `addresses`. If not, we skip it's value.
                if let Some(amount) = self.balances.get_mut(a) {
                    *amount -= input.amount();
                    if *amount == OutputAmount(0) {
                        self.balances.remove(a);
                    }
                }
            }
        }

        for output in created {
            if let Some(&a) = output.owning_address() {
                // All inputs should be present in `addresses`. If not, we skip it's value.
                *self.balances.entry(a).or_default() += output.amount();
            }
        }
    }

    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(PerMilestone {
            at,
            inner: AddressBalanceMeasurement {
                address_with_balance_count: self.balances.len(),
            },
        })
    }
}
