// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the ledger.

use crate::types::ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp};

mod active_addresses;
mod address_balance;
mod base_token;
mod ledger_outputs;
mod unclaimed_tokens;

pub use self::{
    active_addresses::AddressActivity,
    address_balance::AddressBalanceAnalytics,
    base_token::BaseTokenActivityAnalytics,
    ledger_outputs::{CountValue, LedgerOutputAnalytics},
    unclaimed_tokens::UnclaimedTokenAnalytics,
};

#[allow(missing_docs)]
pub trait TransactionAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, at: MilestoneIndexTimestamp);
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]);
    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement>;
}
