// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::output::Output;
use serde::{Deserialize, Serialize};

use super::CountAndAmount;
use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::ledger::{LedgerOutput, LedgerSpent},
};

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub(crate) struct UnlockConditionMeasurement {
    pub(crate) timelock: CountAndAmount,
    pub(crate) expiration: CountAndAmount,
    pub(crate) storage_deposit_return: CountAndAmount,
    pub(crate) storage_deposit_return_inner_amount: u64,
}

impl UnlockConditionMeasurement {
    fn wrapping_add(&mut self, rhs: Self) {
        self.timelock.wrapping_add(rhs.timelock);
        self.expiration.wrapping_add(rhs.expiration);
        self.storage_deposit_return.wrapping_add(rhs.storage_deposit_return);
        self.storage_deposit_return_inner_amount = self
            .storage_deposit_return_inner_amount
            .wrapping_add(rhs.storage_deposit_return_inner_amount);
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        self.timelock.wrapping_sub(rhs.timelock);
        self.expiration.wrapping_sub(rhs.expiration);
        self.storage_deposit_return.wrapping_sub(rhs.storage_deposit_return);
        self.storage_deposit_return_inner_amount = self
            .storage_deposit_return_inner_amount
            .wrapping_sub(rhs.storage_deposit_return_inner_amount);
    }

    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            match output.output() {
                Output::Basic(basic) => {
                    if basic.unlock_conditions().timelock().is_some() {
                        measurement.timelock.add_output(output);
                    }
                    if basic.unlock_conditions().expiration().is_some() {
                        measurement.expiration.add_output(output);
                    }
                    if let Some(storage) = basic.unlock_conditions().storage_deposit_return() {
                        measurement.storage_deposit_return.add_output(output);
                        measurement.storage_deposit_return_inner_amount += storage.amount();
                    }
                }
                Output::Nft(nft) => {
                    if nft.unlock_conditions().timelock().is_some() {
                        measurement.timelock.add_output(output);
                    }
                    if nft.unlock_conditions().expiration().is_some() {
                        measurement.expiration.add_output(output);
                    }
                    if let Some(storage) = nft.unlock_conditions().storage_deposit_return() {
                        measurement.storage_deposit_return.add_output(output);
                        measurement.storage_deposit_return_inner_amount += storage.amount();
                    }
                }
                _ => {}
            }
        }
        measurement
    }
}

impl Analytics for UnlockConditionMeasurement {
    type Measurement = Self;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        self.wrapping_add(created);
        self.wrapping_sub(consumed);
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        *self
    }
}
