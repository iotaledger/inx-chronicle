// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use iota_types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::types::stardust::tangle::{block::Address, milestone::MilestoneTimestamp};

/// Defines a unix time until which only Address, defined in Address Unlock Condition, is allowed to unlock the output.
/// After or at the unix time, only Return Address can unlock it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpirationUnlockCondition {
    return_address: Address,
    timestamp: MilestoneTimestamp,
}

impl<T: Borrow<iota::ExpirationUnlockCondition>> From<T> for ExpirationUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            return_address: value.borrow().return_address().into(),
            timestamp: value.borrow().timestamp().into(),
        }
    }
}

impl TryFrom<ExpirationUnlockCondition> for iota::ExpirationUnlockCondition {
    type Error = iota_types::block::Error;

    fn try_from(value: ExpirationUnlockCondition) -> Result<Self, Self::Error> {
        iota::ExpirationUnlockCondition::new(value.return_address.into(), value.timestamp.0)
    }
}

impl From<ExpirationUnlockCondition> for iota::dto::ExpirationUnlockConditionDto {
    fn from(value: ExpirationUnlockCondition) -> Self {
        Self {
            kind: iota::ExpirationUnlockCondition::KIND,
            return_address: value.return_address.into(),
            timestamp: value.timestamp.0,
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::number::rand_number;

    use super::*;

    impl ExpirationUnlockCondition {
        /// Generates a random [`ExpirationUnlockCondition`].
        pub fn rand() -> Self {
            Self {
                return_address: Address::rand_ed25519(),
                timestamp: rand_number::<u32>().into(),
            }
        }
    }
}
