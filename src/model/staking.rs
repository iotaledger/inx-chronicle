// Copyright 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the staking feature.

use core::borrow::Borrow;

use iota_sdk::types::block::{output::feature::StakingFeature, slot::EpochIndex};
use serde::{Deserialize, Serialize};

/// A native token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingFeatureDto {
    /// The amount of coins that are locked and staked in the containing account.
    pub staked_amount: u64,
    /// The fixed cost of the validator, which it receives as part of its Mana rewards.
    pub fixed_cost: u64,
    /// The epoch index in which the staking started.
    pub start_epoch: EpochIndex,
    /// The epoch index in which the staking ends.
    pub end_epoch: EpochIndex,
}

impl<T: Borrow<StakingFeature>> From<T> for StakingFeatureDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            staked_amount: value.staked_amount(),
            fixed_cost: value.fixed_cost(),
            start_epoch: value.start_epoch(),
            end_epoch: value.end_epoch(),
        }
    }
}

impl From<StakingFeatureDto> for StakingFeature {
    fn from(value: StakingFeatureDto) -> Self {
        Self::new(
            value.staked_amount,
            value.fixed_cost,
            value.start_epoch,
            value.end_epoch,
        )
    }
}
