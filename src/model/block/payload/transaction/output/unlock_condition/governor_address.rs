// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_sdk::types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::model::utxo::AddressDto;

/// Defines the Governor Address that owns this output, that is, it can unlock it with the proper Unlock in a
/// transaction that governance transitions the alias output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernorAddressUnlockConditionDto {
    /// The associated address of this Governor Address Unlock Condition.
    pub address: AddressDto,
}

impl<T: Borrow<iota::GovernorAddressUnlockCondition>> From<T> for GovernorAddressUnlockConditionDto {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}
