// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::{
    types::block::{
        output::{
            feature::{NativeTokenFeature, StakingFeature},
            Feature,
        },
        payload::SignedTransactionPayload,
    },
    utils::serde::string,
    U256,
};
use serde::{Deserialize, Serialize};

use super::CountAndAmount;
use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::ledger::{LedgerOutput, LedgerSpent},
};

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub(crate) struct FeaturesMeasurement {
    pub(crate) native_tokens: NativeTokensCountAndAmount,
    pub(crate) block_issuer: CountAndAmount,
    pub(crate) staking: StakingCountAndAmount,
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
                        Feature::NativeToken(nt) => measurement.native_tokens.add_native_token(nt),
                        Feature::BlockIssuer(_) => measurement.block_issuer.add_output(output),
                        Feature::Staking(staking) => measurement.staking.add_staking(staking),
                        _ => (),
                    }
                }
            }
        }
        measurement
    }
}

#[async_trait::async_trait]
impl Analytics for FeaturesMeasurement {
    type Measurement = Self;

    async fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        _ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        self.wrapping_add(created);
        self.wrapping_sub(consumed);

        Ok(())
    }

    async fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        Ok(*self)
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct NativeTokensCountAndAmount {
    pub(crate) count: usize,
    #[serde(with = "string")]
    pub(crate) amount: U256,
}

impl NativeTokensCountAndAmount {
    fn wrapping_add(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_add(rhs.count),
            amount: self.amount.overflowing_add(rhs.amount).0,
        }
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_sub(rhs.count),
            amount: self.amount.overflowing_sub(rhs.amount).0,
        }
    }

    fn add_native_token(&mut self, nt: &NativeTokenFeature) {
        self.count += 1;
        self.amount += nt.amount();
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct StakingCountAndAmount {
    pub(crate) count: usize,
    #[serde(with = "string")]
    pub(crate) staked_amount: u64,
}

impl StakingCountAndAmount {
    fn wrapping_add(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_add(rhs.count),
            staked_amount: self.staked_amount.wrapping_add(rhs.staked_amount),
        }
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_sub(rhs.count),
            staked_amount: self.staked_amount.wrapping_sub(rhs.staked_amount),
        }
    }

    fn add_staking(&mut self, staking: &StakingFeature) {
        self.count += 1;
        self.staked_amount += staking.staked_amount();
    }
}
