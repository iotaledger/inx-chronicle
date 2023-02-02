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
    active_addresses::{AddressActivityAnalytics, AddressActivityMeasurement},
    address_balance::{AddressBalanceMeasurement, AddressBalancesAnalytics},
    base_token::BaseTokenActivityMeasurement,
    ledger_outputs::{CountValue, LedgerOutputMeasurement},
    ledger_size::{LedgerSizeAnalytics, LedgerSizeMeasurement},
    output_activity::OutputActivityMeasurement,
    unclaimed_tokens::UnclaimedTokenMeasurement,
    unlock_conditions::UnlockConditionMeasurement,
};

#[allow(missing_docs)]
pub trait TransactionAnalytics {
    type Measurement;
    fn begin_milestone(&mut self, at: MilestoneIndexTimestamp);
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]);
    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement>;
}
