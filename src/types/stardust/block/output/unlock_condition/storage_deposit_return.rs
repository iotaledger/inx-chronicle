// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use super::OutputAmount;
use crate::types::stardust::block::Address;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDepositReturnUnlockCondition {
    return_address: Address,
    amount: OutputAmount,
}

impl From<&bee::StorageDepositReturnUnlockCondition> for StorageDepositReturnUnlockCondition {
    fn from(value: &bee::StorageDepositReturnUnlockCondition) -> Self {
        Self {
            return_address: value.return_address().into(),
            amount: value.amount().into(),
        }
    }
}

impl TryFrom<StorageDepositReturnUnlockCondition> for bee::StorageDepositReturnUnlockCondition {
    type Error = bee_block_stardust::Error;

    fn try_from(value: StorageDepositReturnUnlockCondition) -> Result<Self, Self::Error> {
        bee::StorageDepositReturnUnlockCondition::new(value.return_address.into(), value.amount.0)
    }
}
