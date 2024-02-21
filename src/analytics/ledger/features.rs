// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{output::Feature, payload::SignedTransactionPayload};
use serde::{Deserialize, Serialize};

use super::CountAndAmount;
use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::ledger::{LedgerOutput, LedgerSpent},
};

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub(crate) struct FeaturesMeasurement {
    pub(crate) native_tokens: CountAndAmount,
    pub(crate) block_issuer: CountAndAmount,
    pub(crate) staking: CountAndAmount,
}

impl FeaturesMeasurement {
    fn wrapping_add(&mut self, rhs: Self) {
        self.native_tokens.wrapping_add(rhs.native_tokens);
        self.block_issuer.wrapping_add(rhs.block_issuer);
        self.staking.wrapping_add(rhs.staking);
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        self.native_tokens.wrapping_sub(rhs.native_tokens);
        self.block_issuer.wrapping_sub(rhs.block_issuer);
        self.staking.wrapping_sub(rhs.staking);
    }

    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            if let Some(features) = output.output().features() {
                for feature in features.iter() {
                    match feature {
                        Feature::NativeToken(_) => measurement.native_tokens.add_output(output),
                        Feature::BlockIssuer(_) => measurement.block_issuer.add_output(output),
                        Feature::Staking(_) => measurement.staking.add_output(output),
                        _ => (),
                    }
                }
            }
        }
        measurement
    }
}

impl Analytics for FeaturesMeasurement {
    type Measurement = Self;

    fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        _ctx: &dyn AnalyticsContext,
    ) {
        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        self.wrapping_add(created);
        self.wrapping_sub(consumed);
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        *self
    }
}
