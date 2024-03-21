// Copyright 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the storage deposit return unlock condition.

use core::borrow::Borrow;

use iota_sdk::types::block::output::unlock_condition::{StorageDepositReturnUnlockCondition, UnlockConditionError};
use serde::{Deserialize, Serialize};

use super::address::AddressDto;

/// A native token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDepositReturnUnlockConditionDto {
    /// The address to return the amount to.
    pub return_address: AddressDto,
    /// Amount of IOTA coins the consuming transaction should deposit to `return_address`.
    pub amount: u64,
}

impl<T: Borrow<StorageDepositReturnUnlockCondition>> From<T> for StorageDepositReturnUnlockConditionDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            return_address: value.return_address().into(),
            amount: value.amount(),
        }
    }
}

impl TryFrom<StorageDepositReturnUnlockConditionDto> for StorageDepositReturnUnlockCondition {
    type Error = UnlockConditionError;

    fn try_from(value: StorageDepositReturnUnlockConditionDto) -> Result<Self, Self::Error> {
        Self::new(value.return_address, value.amount)
    }
}
