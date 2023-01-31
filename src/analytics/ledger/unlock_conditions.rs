// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{CountValue, TransactionAnalytics};
use crate::{
    db::collections::analytics::UnlockConditionAnalyticsResult,
    types::{
        ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::Output,
    },
};

#[derive(Clone, Debug, Default)]
#[allow(missing_docs)]
pub struct UnlockConditionAnalytics {
    pub timelock: CountValue,
    pub expiration: CountValue,
    pub storage_deposit_return: CountValue,
    pub storage_deposit_return_inner_value: u64,
}

impl UnlockConditionAnalytics {
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

impl TransactionAnalytics for UnlockConditionAnalytics {
    type Measurement = UnlockConditionAnalyticsResult;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        for input in consumed {
            match &input.output.output {
                Output::Basic(basic) => {
                    if basic.timelock_unlock_condition.is_some() {
                        self.timelock -= input;
                    }
                    if basic.expiration_unlock_condition.is_some() {
                        self.expiration -= input;
                    }
                    if let Some(storage) = basic.storage_deposit_return_unlock_condition {
                        self.storage_deposit_return -= input;
                        self.storage_deposit_return_inner_value -= storage.amount.0;
                    }
                }
                Output::Nft(nft) => {
                    if nft.timelock_unlock_condition.is_some() {
                        self.timelock -= input;
                    }
                    if nft.expiration_unlock_condition.is_some() {
                        self.expiration -= input;
                    }
                    if let Some(storage) = nft.storage_deposit_return_unlock_condition {
                        self.storage_deposit_return -= input;
                        self.storage_deposit_return_inner_value -= storage.amount.0;
                    }
                }
                _ => {}
            }
        }

        for output in created {
            match &output.output {
                Output::Basic(basic) => {
                    if basic.timelock_unlock_condition.is_some() {
                        self.timelock += output;
                    }
                    if basic.expiration_unlock_condition.is_some() {
                        self.expiration += output;
                    }
                    if let Some(storage) = basic.storage_deposit_return_unlock_condition {
                        self.storage_deposit_return += output;
                        self.storage_deposit_return_inner_value += storage.amount.0;
                    }
                }
                Output::Nft(nft) => {
                    if nft.timelock_unlock_condition.is_some() {
                        self.timelock += output;
                    }
                    if nft.expiration_unlock_condition.is_some() {
                        self.expiration += output;
                    }
                    if let Some(storage) = nft.storage_deposit_return_unlock_condition {
                        self.storage_deposit_return += output;
                        self.storage_deposit_return_inner_value += storage.amount.0;
                    }
                }
                _ => {}
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(UnlockConditionAnalyticsResult {
            timelock_count: self.timelock.count as _,
            timelock_value: self.timelock.value,
            expiration_count: self.expiration.count as _,
            expiration_value: self.expiration.value,
            storage_deposit_return_count: self.storage_deposit_return.count as _,
            storage_deposit_return_value: self.storage_deposit_return.value,
            storage_deposit_return_inner_value: self.storage_deposit_return_inner_value,
        })
    }
}
