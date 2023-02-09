// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Copy, Clone, Debug, Default, AddAssign, SubAssign)]
#[allow(missing_docs)]
pub(crate) struct UnlockConditionMeasurement {
    pub(crate) timelock: CountAndAmount,
    pub(crate) expiration: CountAndAmount,
    pub(crate) storage_deposit_return: CountAndAmount,
    pub(crate) storage_deposit_return_inner_value: u64,
}

impl UnlockConditionMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
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

impl Analytics for UnlockConditionMeasurement {
    type Measurement = PerMilestone<Self>;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        *self += created;
        *self -= consumed;
    }

    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        Some(PerMilestone {
            at: *ctx.at(),
            inner: *self,
        })
    }
}
