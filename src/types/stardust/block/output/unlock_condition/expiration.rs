// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::{block::Address, milestone::MilestoneTimestamp};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpirationUnlockCondition {
    return_address: Address,
    timestamp: MilestoneTimestamp,
}

impl From<&bee::ExpirationUnlockCondition> for ExpirationUnlockCondition {
    fn from(value: &bee::ExpirationUnlockCondition) -> Self {
        Self {
            return_address: value.return_address().into(),
            timestamp: value.timestamp().into(),
        }
    }
}

impl TryFrom<ExpirationUnlockCondition> for bee::ExpirationUnlockCondition {
    type Error = bee_block_stardust::Error;

    fn try_from(value: ExpirationUnlockCondition) -> Result<Self, Self::Error> {
        bee::ExpirationUnlockCondition::new(value.return_address.into(), value.timestamp.0)
    }
}
