// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_sdk::types::block::{address::Address, output::unlock_condition as iota};
use serde::{Deserialize, Serialize};

use crate::model::utxo::AddressDto;

/// Defines the permanent alias address that owns this output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImmutableAccountAddressUnlockConditionDto {
    /// The associated address of this Immutable Account Address Unlock Condition
    pub address: AddressDto,
}

impl<T: Borrow<iota::ImmutableAccountAddressUnlockCondition>> From<T> for ImmutableAccountAddressUnlockConditionDto {
    fn from(value: T) -> Self {
        Self {
            address: Address::from(*value.borrow().address()).into(),
        }
    }
}
