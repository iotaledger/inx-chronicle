// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_sdk::types::block::{output::unlock_condition as iota, slot::SlotIndex};
use serde::{Deserialize, Serialize};

use crate::model::utxo::AddressDto;

/// Defines a unix time until which only Address, defined in Address Unlock Condition, is allowed to unlock the output.
/// After or at the unix time, only Return Address can unlock it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpirationUnlockConditionDto {
    pub return_address: AddressDto,
    pub slot_index: SlotIndex,
}

impl<T: Borrow<iota::ExpirationUnlockCondition>> From<T> for ExpirationUnlockConditionDto {
    fn from(value: T) -> Self {
        Self {
            return_address: value.borrow().return_address().into(),
            slot_index: value.borrow().slot_index(),
        }
    }
}
