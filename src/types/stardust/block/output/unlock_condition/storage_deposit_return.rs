// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use super::OutputAmount;
use crate::types::stardust::block::Address;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDepositReturnUnlockCondition {
    return_address: Address,
    amount: OutputAmount,
}

impl<T: Borrow<bee::StorageDepositReturnUnlockCondition>> From<T> for StorageDepositReturnUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            return_address: value.borrow().return_address().into(),
            amount: value.borrow().amount().into(),
        }
    }
}

impl TryFrom<StorageDepositReturnUnlockCondition> for bee::StorageDepositReturnUnlockCondition {
    type Error = bee_block_stardust::Error;

    fn try_from(value: StorageDepositReturnUnlockCondition) -> Result<Self, Self::Error> {
        bee::StorageDepositReturnUnlockCondition::new(value.return_address.into(), value.amount.0)
    }
}

#[cfg(feature = "rand")]
mod rand {
    use super::*;

    impl StorageDepositReturnUnlockCondition {
        /// Generates a random [`StorageDepositReturnUnlockCondition`].
        pub fn rand() -> Self {
            Self {
                return_address: Address::rand_ed25519(),
                amount: OutputAmount::rand(),
            }
        }
    }
}
