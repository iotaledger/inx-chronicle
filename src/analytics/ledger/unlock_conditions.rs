// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use derive_more::{AddAssign, SubAssign};

use super::{CountValue, TransactionAnalytics};
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
    stardust::block::Output,
};

#[derive(Copy, Clone, Debug, Default, AddAssign, SubAssign)]
#[allow(missing_docs)]
pub struct UnlockConditionMeasurement {
    pub timelock: CountValue,
    pub expiration: CountValue,
    pub storage_deposit_return: CountValue,
    pub storage_deposit_return_inner_value: u64,
}

impl UnlockConditionMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            match &output.output {
                Output::Alias(_) => {}
                Output::Basic(basic) => {
                    if basic.timelock_unlock_condition.is_some() {
                        measurement.timelock += output;
                    }
                    if basic.expiration_unlock_condition.is_some() {
                        measurement.expiration += output;
                    }
                    if let Some(storage) = basic.storage_deposit_return_unlock_condition {
                        measurement.storage_deposit_return += output;
                        measurement.storage_deposit_return_inner_value += storage.amount.0;
                    }
                }
                Output::Nft(nft) => {
                    if nft.timelock_unlock_condition.is_some() {
                        measurement.timelock += output;
                    }
                    if nft.expiration_unlock_condition.is_some() {
                        measurement.expiration += output;
                    }
                    if let Some(storage) = nft.storage_deposit_return_unlock_condition {
                        measurement.storage_deposit_return += output;
                        measurement.storage_deposit_return_inner_value += storage.amount.0;
                    }
                }
                Output::Foundry(_) => {}
                Output::Treasury(_) => {}
            }
        }
        measurement
    }
}

impl TransactionAnalytics for UnlockConditionMeasurement {
    type Measurement = Self;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        *self += created;
        *self -= consumed;
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(*self)
    }
}
