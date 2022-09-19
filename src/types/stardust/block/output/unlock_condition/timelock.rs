// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::milestone::MilestoneTimestamp;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelockUnlockCondition {
    timestamp: MilestoneTimestamp,
}

impl<T: Borrow<bee::TimelockUnlockCondition>> From<T> for TimelockUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            timestamp: value.borrow().timestamp().into(),
        }
    }
}

impl TryFrom<TimelockUnlockCondition> for bee::TimelockUnlockCondition {
    type Error = bee_block_stardust::Error;

    fn try_from(value: TimelockUnlockCondition) -> Result<Self, Self::Error> {
        Self::new(value.timestamp.0)
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::number::rand_number;

    use super::*;

    impl TimelockUnlockCondition {
        /// Generates a random [`TimelockUnlockCondition`].
        pub fn rand() -> Self {
            Self {
                timestamp: rand_number::<u32>().into(),
            }
        }
    }
}
