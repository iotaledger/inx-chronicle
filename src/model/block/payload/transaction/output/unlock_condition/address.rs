// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_sdk::types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::model::utxo::AddressDto;

/// Defines the Address that owns an output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressUnlockConditionDto {
    /// The associated address of this Address Unlock Condition
    pub address: AddressDto,
}

impl<T: Borrow<iota::AddressUnlockCondition>> From<T> for AddressUnlockConditionDto {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}
