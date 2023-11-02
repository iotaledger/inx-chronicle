// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::borrow::Borrow;

use iota_sdk::{
    types::block::output::{self as iota, AnchorId},
    utils::serde::string,
};
use serde::{Deserialize, Serialize};

use super::{
    unlock_condition::{GovernorAddressUnlockConditionDto, StateControllerAddressUnlockConditionDto},
    FeatureDto, NativeTokenDto,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorOutputDto {
    /// Amount of IOTA coins held by the output.
    #[serde(with = "string")]
    pub amount: u64,
    // Amount of mana held by the output.
    #[serde(with = "string")]
    pub mana: u64,
    /// Native tokens held by the output.
    pub native_tokens: Vec<NativeTokenDto>,
    /// Unique identifier of the anchor.
    pub anchor_id: AnchorId,
    /// A counter that must increase by 1 every time the anchor is state transitioned.
    pub state_index: u32,
    /// Metadata that can only be changed by the state controller.
    #[serde(with = "serde_bytes")]
    pub state_metadata: Box<[u8]>,
    /// The state controller unlock condition.
    pub state_controller_unlock_condition: StateControllerAddressUnlockConditionDto,
    /// The governor unlock condition.
    pub governor_unlock_condition: GovernorAddressUnlockConditionDto,
    /// Features of the output.
    pub features: Vec<FeatureDto>,
    /// Immutable features of the output.
    pub immutable_features: Vec<FeatureDto>,
}

impl AnchorOutputDto {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "basic";
}

impl<T: Borrow<iota::AnchorOutput>> From<T> for AnchorOutputDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount(),
            mana: value.mana(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            anchor_id: *value.anchor_id(),
            state_index: value.state_index(),
            state_metadata: value.state_metadata().into(),
            state_controller_unlock_condition: StateControllerAddressUnlockConditionDto {
                address: value.state_controller_address().into(),
            },
            governor_unlock_condition: GovernorAddressUnlockConditionDto {
                address: value.governor_address().into(),
            },
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}
