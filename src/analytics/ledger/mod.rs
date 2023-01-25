// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the ledger.

use crate::types::{
    ledger::{LedgerOutput, LedgerSpent},
    tangle::MilestoneIndex,
};

mod address_balance;
mod base_token;
mod ledger_size;
mod unclaimed_tokens;

pub use self::{
    address_balance::{AddressBalanceAnalytics, AddressCount},
    base_token::BaseTokenActivityAnalytics,
    unclaimed_tokens::{UnclaimedTokens, UnclaimedTokensAnalytics},
};

#[allow(missing_docs)]
pub trait TransactionAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, index: MilestoneIndex);
    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]);
    fn end_milestone(&mut self, index: MilestoneIndex) -> Option<Self::Measurement>;
}
