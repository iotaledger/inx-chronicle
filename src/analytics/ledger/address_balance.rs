// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use iota_sdk::types::block::{
    address::{AccountAddress, Address, AnchorAddress, Ed25519Address, ImplicitAccountCreationAddress, NftAddress},
    payload::SignedTransactionPayload,
};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::ledger::{LedgerOutput, LedgerSpent},
};

#[derive(Debug)]
pub(crate) struct AddressBalanceMeasurement {
    pub(crate) ed25519_address_with_balance_count: usize,
    pub(crate) account_address_with_balance_count: usize,
    pub(crate) nft_address_with_balance_count: usize,
    pub(crate) anchor_address_with_balance_count: usize,
    pub(crate) implicit_address_with_balance_count: usize,
    pub(crate) token_distribution: Vec<DistributionStat>,
}

/// Statistics for a particular logarithmic range of balances.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct DistributionStat {
    pub(crate) ed25519_count: usize,
    pub(crate) ed25519_amount: u64,
    pub(crate) account_count: usize,
    pub(crate) account_amount: u64,
    pub(crate) nft_count: usize,
    pub(crate) nft_amount: u64,
    pub(crate) anchor_count: usize,
    pub(crate) anchor_amount: u64,
    pub(crate) implicit_count: usize,
    pub(crate) implicit_amount: u64,
}

/// Computes the number of addresses the currently hold a balance.
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct AddressBalancesAnalytics {
    ed25519_balances: HashMap<Ed25519Address, u64>,
    account_balances: HashMap<AccountAddress, u64>,
    nft_balances: HashMap<NftAddress, u64>,
    anchor_balances: HashMap<AnchorAddress, u64>,
    implicit_balances: HashMap<ImplicitAccountCreationAddress, u64>,
}

impl AddressBalancesAnalytics {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut balances = AddressBalancesAnalytics::default();
        for output in unspent_outputs {
            if let Some(a) = output.address() {
                balances.add_address(a, output.amount());
            }
        }
        balances
    }

    fn add_address(&mut self, address: &Address, output_amount: u64) {
        match address {
            Address::Ed25519(a) => *self.ed25519_balances.entry(*a).or_default() += output_amount,
            Address::Account(a) => *self.account_balances.entry(*a).or_default() += output_amount,
            Address::Nft(a) => *self.nft_balances.entry(*a).or_default() += output_amount,
            Address::Anchor(a) => *self.anchor_balances.entry(*a).or_default() += output_amount,
            Address::ImplicitAccountCreation(a) => *self.implicit_balances.entry(*a).or_default() += output_amount,
            _ => (),
        }
    }

    fn remove_amount(&mut self, address: &Address, output_amount: u64) {
        match address {
            Address::Ed25519(a) => {
                if let Some(amount) = self.ed25519_balances.get_mut(a) {
                    *amount -= output_amount;
                    if *amount == 0 {
                        self.ed25519_balances.remove(a);
                    }
                }
            }
            Address::Account(a) => {
                if let Some(amount) = self.account_balances.get_mut(a) {
                    *amount -= output_amount;
                    if *amount == 0 {
                        self.account_balances.remove(a);
                    }
                }
            }
            Address::Nft(a) => {
                if let Some(amount) = self.nft_balances.get_mut(a) {
                    *amount -= output_amount;
                    if *amount == 0 {
                        self.nft_balances.remove(a);
                    }
                }
            }
            Address::Anchor(a) => {
                if let Some(amount) = self.anchor_balances.get_mut(a) {
                    *amount -= output_amount;
                    if *amount == 0 {
                        self.anchor_balances.remove(a);
                    }
                }
            }
            Address::ImplicitAccountCreation(a) => {
                if let Some(amount) = self.implicit_balances.get_mut(a) {
                    *amount -= output_amount;
                    if *amount == 0 {
                        self.implicit_balances.remove(a);
                    }
                }
            }
            _ => (),
        }
    }
}

impl Analytics for AddressBalancesAnalytics {
    type Measurement = AddressBalanceMeasurement;

    fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        _ctx: &dyn AnalyticsContext,
    ) {
        for output in consumed {
            if let Some(address) = output.address() {
                self.remove_amount(address, output.amount());
            }
        }

        for output in created {
            if let Some(address) = output.address() {
                self.add_address(address, output.amount())
            }
        }
    }

    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Self::Measurement {
        let bucket_max = ctx.protocol_parameters().token_supply().ilog10() as usize + 1;
        let mut token_distribution = vec![DistributionStat::default(); bucket_max];

        // Balances are partitioned into ranges defined by: [10^index..10^(index+1)).
        for amount in self.ed25519_balances.values() {
            let index = amount.ilog10() as usize;
            token_distribution[index].ed25519_count += 1;
            token_distribution[index].ed25519_amount += *amount;
        }
        for amount in self.account_balances.values() {
            let index = amount.ilog10() as usize;
            token_distribution[index].account_count += 1;
            token_distribution[index].account_amount += *amount;
        }
        for amount in self.nft_balances.values() {
            let index = amount.ilog10() as usize;
            token_distribution[index].nft_count += 1;
            token_distribution[index].nft_amount += *amount;
        }
        for amount in self.anchor_balances.values() {
            let index = amount.ilog10() as usize;
            token_distribution[index].anchor_count += 1;
            token_distribution[index].anchor_amount += *amount;
        }
        for amount in self.implicit_balances.values() {
            let index = amount.ilog10() as usize;
            token_distribution[index].implicit_count += 1;
            token_distribution[index].implicit_amount += *amount;
        }
        AddressBalanceMeasurement {
            ed25519_address_with_balance_count: self.ed25519_balances.len(),
            account_address_with_balance_count: self.account_balances.len(),
            nft_address_with_balance_count: self.nft_balances.len(),
            anchor_address_with_balance_count: self.anchor_balances.len(),
            implicit_address_with_balance_count: self.implicit_balances.len(),
            token_distribution,
        }
    }
}
