// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use iota_sdk::types::block::{
    address::{Bech32Address, ToBech32Ext},
    protocol::ProtocolParameters,
};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::ledger::{LedgerOutput, LedgerSpent},
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
    pub(crate) total_amount: u64,
}

/// Computes the number of addresses the currently hold a balance.
#[derive(Serialize, Deserialize)]
pub(crate) struct AddressBalancesAnalytics {
    balances: HashMap<Bech32Address, u64>,
}

impl AddressBalancesAnalytics {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
        protocol_params: &ProtocolParameters,
    ) -> Self {
        let hrp = protocol_params.bech32_hrp();
        let mut balances = HashMap::new();
        for output in unspent_outputs {
            if let Some(a) = output.address() {
                *balances.entry(a.clone().to_bech32(hrp)).or_default() += output.amount();
            }
        }
        Self { balances }
    }
}

impl Analytics for AddressBalancesAnalytics {
    type Measurement = AddressBalanceMeasurement;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], ctx: &dyn AnalyticsContext) {
        let hrp = ctx.protocol_params().bech32_hrp();
        for output in consumed {
            if let Some(a) = output.address() {
                let a = a.clone().to_bech32(hrp);
                // All inputs should be present in `addresses`. If not, we skip it's value.
                if let Some(amount) = self.balances.get_mut(&a) {
                    *amount -= output.amount();
                    if *amount == 0 {
                        self.balances.remove(&a);
                    }
                }
            }
        }

        for output in created {
            if let Some(a) = output.address() {
                // All inputs should be present in `addresses`. If not, we skip it's value.
                *self.balances.entry(a.clone().to_bech32(hrp)).or_default() += output.amount();
            }
        }
    }

    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Self::Measurement {
        let bucket_max = ctx.protocol_params().token_supply().ilog10() as usize + 1;
        let mut token_distribution = vec![DistributionStat::default(); bucket_max];

        for amount in self.balances.values() {
            // Balances are partitioned into ranges defined by: [10^index..10^(index+1)).
            let index = amount.ilog10() as usize;
            token_distribution[index].address_count += 1;
            token_distribution[index].total_amount += *amount;
        }
        AddressBalanceMeasurement {
            address_with_balance_count: self.balances.len(),
            token_distribution,
        }
    }
}
