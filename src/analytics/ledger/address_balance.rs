// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::*;
use crate::model::utxo::{Address, TokenAmount};

#[derive(Debug)]
pub(crate) struct AddressesWithBalanceMeasurement {
    pub(crate) address_with_balance_count: usize,
    pub(crate) token_distribution: Vec<DistributionStat>,
}

/// Statistics for a particular logarithmic range of balances.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct DistributionStat {
    /// The number of unique addresses in this range.
    pub(crate) address_count: u64,
    /// The total amount of tokens in this range.
    pub(crate) total_amount: TokenAmount,
}

/// Statistics for an address's balance.
#[derive(Clone, Debug)]
pub(crate) struct AddressBalanceMeasurement {
    pub(crate) address: String,
    pub(crate) balance: TokenAmount,
}

/// Computes the number of addresses the currently hold a balance.
#[derive(Serialize, Deserialize)]
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
    type Measurement = (AddressesWithBalanceMeasurement, Vec<AddressBalanceMeasurement>);

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

    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Self::Measurement {
        let bucket_max = ctx.protocol_params().token_supply.ilog10() as usize + 1;
        let mut token_distribution = vec![DistributionStat::default(); bucket_max];

        for amount in self.balances.values() {
            // Balances are partitioned into ranges defined by: [10^index..10^(index+1)).
            let index = amount.0.ilog10() as usize;
            token_distribution[index].address_count += 1;
            token_distribution[index].total_amount += *amount;
        }

        let mut sorted_addresses = self.balances.iter().collect::<Vec<_>>();
        sorted_addresses.sort_unstable_by_key(|(_, v)| v.0);
        let bech32_hrp = &ctx.protocol_params().bech32_hrp;
        (
            AddressesWithBalanceMeasurement {
                address_with_balance_count: self.balances.len(),
                token_distribution,
            },
            sorted_addresses
                .iter()
                .rev()
                .take(100)
                .map(|(a, v)| AddressBalanceMeasurement {
                    address: a.to_bech32(bech32_hrp),
                    balance: **v,
                })
                .collect(),
        )
    }
}
