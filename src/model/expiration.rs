// Copyright 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the expiration unlock condition.

use core::borrow::Borrow;

use iota_sdk::types::block::{output::unlock_condition::ExpirationUnlockCondition, slot::SlotIndex};
use serde::{Deserialize, Serialize};

use super::address::AddressDto;

/// A native token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpirationUnlockConditionDto {
    /// The address that can unlock the expired output.
    pub return_address: AddressDto,
    /// The slot index that determines when the associated output expires.
    pub slot_index: SlotIndex,
}

impl<T: Borrow<ExpirationUnlockCondition>> From<T> for ExpirationUnlockConditionDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            return_address: value.return_address().into(),
            slot_index: value.slot_index(),
        }
    }
}

impl TryFrom<ExpirationUnlockConditionDto> for ExpirationUnlockCondition {
    type Error = iota_sdk::types::block::Error;

    fn try_from(value: ExpirationUnlockConditionDto) -> Result<Self, Self::Error> {
        Self::new(value.return_address, value.slot_index)
    }
}
