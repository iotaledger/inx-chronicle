// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Statistics about the ledger.

use std::ops::{AddAssign, SubAssign};

use derive_more::{AddAssign, SubAssign};

pub(crate) use self::{
    active_addresses::{AddressActivityAnalytics, AddressActivityMeasurement},
    address_balance::{AddressBalanceMeasurement, AddressBalancesAnalytics},
    base_token::BaseTokenActivityMeasurement,
    ledger_outputs::LedgerOutputMeasurement,
    ledger_size::{LedgerSizeAnalytics, LedgerSizeMeasurement},
    output_activity::OutputActivityMeasurement,
    unclaimed_tokens::UnclaimedTokenMeasurement,
    unlock_conditions::UnlockConditionMeasurement,
};
use crate::{
    analytics::{
        influx::{PerMilestone, TimeInterval},
        Analytics, AnalyticsContext,
    },
    types::{
        ledger::{LedgerOutput, LedgerSpent},
        stardust::block::Output,
    },
};

mod active_addresses;
mod address_balance;
mod base_token;
mod ledger_outputs;
mod ledger_size;
mod output_activity;
mod unclaimed_tokens;
mod unlock_conditions;

#[derive(Copy, Clone, Debug, Default, AddAssign, SubAssign)]
pub(crate) struct CountValue {
    pub(crate) count: usize,
    pub(crate) value: u64,
}

impl AddAssign<&LedgerOutput> for CountValue {
    fn add_assign(&mut self, rhs: &LedgerOutput) {
        self.count += 1;
        self.value += rhs.output.amount().0;
    }
}

impl SubAssign<&LedgerSpent> for CountValue {
    fn sub_assign(&mut self, rhs: &LedgerSpent) {
        self.count -= 1;
        self.value -= rhs.output.output.amount().0;
    }
}
