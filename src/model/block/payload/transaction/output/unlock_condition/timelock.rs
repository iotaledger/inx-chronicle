// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::model::tangle::MilestoneTimestamp;

/// Defines a unix timestamp until which the output can not be unlocked.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelockUnlockCondition {
    timestamp: MilestoneTimestamp,
}

impl<T: Borrow<iota::TimelockUnlockCondition>> From<T> for TimelockUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            timestamp: value.borrow().timestamp().into(),
        }
    }
}

impl TryFrom<TimelockUnlockCondition> for iota::TimelockUnlockCondition {
    type Error = iota_types::block::Error;

    fn try_from(value: TimelockUnlockCondition) -> Result<Self, Self::Error> {
        Self::new(value.timestamp.0)
    }
}

impl From<TimelockUnlockCondition> for iota::dto::TimelockUnlockConditionDto {
    fn from(value: TimelockUnlockCondition) -> Self {
        Self {
            kind: iota::TimelockUnlockCondition::KIND,
            timestamp: value.timestamp.0,
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::number::rand_number;

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
