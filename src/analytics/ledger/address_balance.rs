// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::*;
use crate::types::stardust::block::{output::TokenAmount, Address};

pub(crate) struct AddressBalanceMeasurement {
    pub(crate) address_with_balance_count: usize,
    pub(crate) distribution: HashMap<u32, DistributionStat>,
}

/// Statistics for a particular logarithmic range of balances.
#[derive(Clone, Debug, Default)]
pub(crate) struct DistributionStat {
    /// The number of unique addresses in this range.
    pub(crate) address_count: u64,
    /// The total amount of tokens in this range.
    pub(crate) total_amount: TokenAmount,
}

/// Computes the number of addresses the currently hold a balance.
pub(crate) struct AddressBalancesAnalytics {
    balances: HashMap<Address, TokenAmount>,
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

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        for output in consumed {
            if let Some(a) = output.owning_address() {
                // All inputs should be present in `addresses`. If not, we skip it's value.
                if let Some(amount) = self.balances.get_mut(a) {
                    *amount -= output.amount();
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

    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        let mut distribution: HashMap<u32, DistributionStat> = (0..=9u32)
            .into_iter()
            .map(|i| (i, DistributionStat::default()))
            .collect();

        for (_, amount) in self.balances.iter() {
            // The logarithmic index the balances are contained between: \[10^index..10^(index+1)\]
            let index = amount.0.ilog10();

            distribution
                .entry(index)
                .and_modify(|stat: &mut DistributionStat| {
                    stat.address_count += 1;
                    stat.total_amount += *amount;
                })
                .or_default();
        }
        Some(PerMilestone {
            at: *ctx.at(),
            inner: AddressBalanceMeasurement {
                address_with_balance_count: self.balances.len(),
                distribution,
            },
        })
    }
}
