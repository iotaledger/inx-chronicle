// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the ledger.

use crate::types::ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp};

mod active_addresses;
mod address_balance;
mod base_token;
mod ledger_outputs;
mod ledger_size;
mod output_activity;
mod unclaimed_tokens;
mod unlock_conditions;

pub(crate) use self::{
    active_addresses::AddressActivity,
    address_balance::AddressBalanceAnalytics,
    base_token::BaseTokenActivityAnalytics,
    ledger_outputs::{CountValue, LedgerOutputAnalytics},
    ledger_size::LedgerSizeAnalytics,
    output_activity::OutputActivityAnalytics,
    unclaimed_tokens::UnclaimedTokenAnalytics,
    unlock_conditions::UnlockConditionAnalytics,
};

#[allow(missing_docs)]
pub trait TransactionAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, at: MilestoneIndexTimestamp);
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]);
    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement>;
}
