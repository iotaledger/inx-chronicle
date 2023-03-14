// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use influxdb::WriteQuery;

use super::*;
use crate::{
    analytics::measurement::Measurement,
    model::utxo::{Address, TokenAmount},
};

#[derive(Debug)]
pub(crate) struct AddressBalanceMeasurement {
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
    type Measurement = AddressBalanceMeasurement;

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
        AddressBalanceMeasurement {
            address_with_balance_count: self.balances.len(),
            token_distribution,
        }
    }
}

impl Measurement for AddressBalanceMeasurement {
    const NAME: &'static str = "stardust_addresses";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        let mut query = query.add_field("address_with_balance_count", self.address_with_balance_count as u64);
        for (index, stat) in self.token_distribution.iter().enumerate() {
            query = query
                .add_field(format!("address_count_{index}"), stat.address_count)
                .add_field(format!("total_amount_{index}"), stat.total_amount.0);
        }
        query
    }
}
