// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::collections::HashMap;

use crate::types::stardust::block::Address;

use super::*;

#[derive(Clone, Debug)]
pub(crate) struct TokenDistributionMeasurement {
    pub(crate) distribution: Vec<DistributionStat>,
}

#[derive(Clone, Debug)]
/// Statistics for a particular logarithmic range of balances
pub(crate) struct DistributionStat {
    /// The logarithmic index the balances are contained between: \[10^index..10^(index+1)\]
    pub(crate) index: u32,
    /// The number of unique addresses in this range
    pub(crate) address_count: u64,
    /// The total balance of the addresses in this range
    pub(crate) total_balance: String,
}

/// Computes the number of addresses the currently hold a balance.
pub(crate) struct TokenDistributionAnalytics {
    balances: HashMap<Address, TokenAmount>,
}

impl TokenDistributionAnalytics {
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

impl Analytics for TokenDistributionAnalytics {
    type Measurement = PerMilestone<TokenDistributionMeasurement>;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        for input in consumed {
            if let Some(a) = input.owning_address() {
                // All inputs should be present in `addresses`. If not, we skip it's value.
                if let Some(amount) = self.balances.get_mut(a) {
                    *amount -= input.amount();
                    if amount.0 == 0 {
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

    fn end_milestone(&mut self, _ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        // for (address, amount) in self.balances.iter() {
        //     let index = amount.0.checked_ilog10()
        // }

        // Some(PerMilestone {
        //     at: *ctx.at(),
        //     inner: AddressBalanceMeasurement {
        //         address_with_balance_count: self.balances.len(),
        //     },
        // })
        todo!()
    }
}
