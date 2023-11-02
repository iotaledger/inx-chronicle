// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_sdk::{types::block::output::unlock_condition as iota, utils::serde::string};
use serde::{Deserialize, Serialize};

use crate::model::utxo::AddressDto;

/// Defines the amount of tokens used as storage deposit that have to be returned to the return address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDepositReturnUnlockConditionDto {
    /// The address to which funds will be returned once the storage deposit is unlocked.
    pub return_address: AddressDto,
    /// The amount held in storage.
    #[serde(with = "string")]
    pub amount: u64,
}

impl<T: Borrow<iota::StorageDepositReturnUnlockCondition>> From<T> for StorageDepositReturnUnlockConditionDto {
    fn from(value: T) -> Self {
        Self {
            return_address: value.borrow().return_address().into(),
            amount: value.borrow().amount().into(),
        }
    }
}
