// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the ledger.

use crate::types::ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp};

mod active_addresses;
mod address_balance;
mod base_token;
mod outputs;
mod unclaimed_tokens;

pub use self::{
    active_addresses::ActiveAddresses,
    address_balance::AddressBalanceAnalytics,
    base_token::BaseTokenActivityAnalytics,
    outputs::{CountValue, OutputState, OutputStatistics},
    unclaimed_tokens::{UnclaimedTokens, UnclaimedTokensAnalytics},
};

#[allow(missing_docs)]
pub trait TransactionAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, at: MilestoneIndexTimestamp);
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]);
    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement>;
}

/// The number of addresses.
pub struct AddressCount(usize);
