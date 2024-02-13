// Copyright 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashMap};

use super::*;
use crate::model::{
    payload::{milestone::MilestoneTimestamp, transaction::output::OutputId},
    utxo::{Address, TokenAmount},
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

/// Computes the number of addresses that currently hold a balance.
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct AddressBalancesAnalytics {
    balances: HashMap<Address, TokenAmount>,
    expiring: BTreeMap<(MilestoneTimestamp, OutputId), (Address, Address, TokenAmount)>,
    locked: BTreeMap<(MilestoneTimestamp, OutputId), (Address, TokenAmount)>,
}

impl AddressBalancesAnalytics {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
        milestone_timestamp: MilestoneTimestamp,
    ) -> Self {
        let mut res = AddressBalancesAnalytics::default();
        for created in unspent_outputs {
            res.handle_created(created, milestone_timestamp);
        }
        res
    }

    fn handle_created(&mut self, created: &LedgerOutput, milestone_timestamp: MilestoneTimestamp) {
        if let Some(&owning_address) = created.owning_address() {
            if let Some(expiration) = created.output.expiration() {
                // If the output is expired already, add the value to the return address
                if milestone_timestamp >= expiration.timestamp {
                    *self.balances.entry(expiration.return_address).or_default() += created.amount();
                } else {
                    // Otherwise, add it to the set of expiring values to be handled later
                    *self.balances.entry(owning_address).or_default() += created.amount();
                    self.expiring.insert(
                        (expiration.timestamp, created.output_id),
                        (owning_address, expiration.return_address, created.amount()),
                    );
                }
            } else if let Some(timelock) = created.output.timelock() {
                // If the output is unlocked, add the value to the address
                if milestone_timestamp >= timelock.timestamp {
                    *self.balances.entry(owning_address).or_default() += created.amount();
                } else {
                    // Otherwise, add it to the set of locked values to be handled later
                    self.locked.insert(
                        (timelock.timestamp, created.output_id),
                        (owning_address, created.amount()),
                    );
                }
            } else {
                *self.balances.entry(owning_address).or_default() += created.amount();
            }
        }
    }

    fn handle_consumed(&mut self, consumed: &LedgerSpent, milestone_timestamp: MilestoneTimestamp) {
        if let Some(&owning_address) = consumed.output.owning_address() {
            if let Some(expiration) = consumed.output.output.expiration() {
                // No longer need to handle the expiration
                self.expiring.remove(&(expiration.timestamp, consumed.output_id()));
                // If the output is past the expiration time, remove the value from the return address
                if milestone_timestamp >= expiration.timestamp {
                    *self.balances.entry(expiration.return_address).or_default() -= consumed.amount();
                // Otherwise, remove it from the original address
                } else {
                    *self.balances.entry(owning_address).or_default() -= consumed.amount();
                }
            } else if let Some(timelock) = consumed.output.output.timelock() {
                // No longer need to handle the lock
                self.locked.remove(&(timelock.timestamp, consumed.output_id()));
                *self.balances.entry(owning_address).or_default() -= consumed.amount();
            } else {
                *self.balances.entry(owning_address).or_default() -= consumed.amount();
            }
        }
    }

    fn handle_expired(&mut self, milestone_timestamp: MilestoneTimestamp) {
        while let Some((address, return_address, amount)) = self.expiring.first_entry().and_then(|entry| {
            if milestone_timestamp >= entry.key().0 {
                Some(entry.remove())
            } else {
                None
            }
        }) {
            *self.balances.entry(address).or_default() -= amount;
            *self.balances.entry(return_address).or_default() += amount;
        }
    }

    fn handle_locked(&mut self, milestone_timestamp: MilestoneTimestamp) {
        while let Some((address, amount)) = self.locked.first_entry().and_then(|entry| {
            if milestone_timestamp >= entry.key().0 {
                Some(entry.remove())
            } else {
                None
            }
        }) {
            *self.balances.entry(address).or_default() += amount;
        }
    }
}

impl Analytics for AddressBalancesAnalytics {
    type Measurement = AddressBalanceMeasurement;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], ctx: &dyn AnalyticsContext) {
        // Handle consumed outputs first, as they can remove entries from expiration/locked
        for consumed in consumed {
            self.handle_consumed(consumed, ctx.at().milestone_timestamp);
        }
        // Handle any expiring or unlocking outputs for this milestone
        self.handle_expired(ctx.at().milestone_timestamp);
        self.handle_locked(ctx.at().milestone_timestamp);
        // Finally, handle the created transactions, which can insert new expiration/locked records for the future
        for created in created {
            self.handle_created(created, ctx.at().milestone_timestamp);
        }
    }

    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Self::Measurement {
        let bucket_max = ctx.protocol_params().token_supply.ilog10() as usize + 1;
        let mut token_distribution = vec![DistributionStat::default(); bucket_max];

        for balance in self.balances.values() {
            // Balances are partitioned into ranges defined by: [10^index..10^(index+1)).
            let index = balance.ilog10() as usize;
            token_distribution[index].address_count += 1;
            token_distribution[index].total_amount += *balance;
        }
        AddressBalanceMeasurement {
            address_with_balance_count: self.balances.len(),
            token_distribution,
        }
    }
}
