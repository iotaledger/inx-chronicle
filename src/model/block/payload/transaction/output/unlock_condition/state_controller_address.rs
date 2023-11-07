// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_sdk::types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::model::utxo::AddressDto;

/// Defines the State Controller Address that owns this output, that is, it can unlock it with the proper Unlock in a
/// transaction that state transitions the alias output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateControllerAddressUnlockConditionDto {
    /// The associated address of this State Controller Address Unlock Condition.
    pub address: AddressDto,
}

impl<T: Borrow<iota::StateControllerAddressUnlockCondition>> From<T> for StateControllerAddressUnlockConditionDto {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}
