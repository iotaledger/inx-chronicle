// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_sdk::types::block::{output::unlock_condition as iota, slot::SlotIndex};
use serde::{Deserialize, Serialize};

/// Defines a unix timestamp until which the output can not be unlocked.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelockUnlockConditionDto {
    slot_index: SlotIndex,
}

impl<T: Borrow<iota::TimelockUnlockCondition>> From<T> for TimelockUnlockConditionDto {
    fn from(value: T) -> Self {
        Self {
            slot_index: value.borrow().slot_index(),
        }
    }
}
