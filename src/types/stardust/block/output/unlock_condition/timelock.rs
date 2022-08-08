// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::milestone::MilestoneTimestamp;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelockUnlockCondition {
    timestamp: MilestoneTimestamp,
}

impl From<&bee::TimelockUnlockCondition> for TimelockUnlockCondition {
    fn from(value: &bee::TimelockUnlockCondition) -> Self {
        Self {
            timestamp: value.timestamp().into(),
        }
    }
}

impl TryFrom<TimelockUnlockCondition> for bee::TimelockUnlockCondition {
    type Error = bee_block_stardust::Error;

    fn try_from(value: TimelockUnlockCondition) -> Result<Self, Self::Error> {
        Self::new(value.timestamp.0)
    }
}
