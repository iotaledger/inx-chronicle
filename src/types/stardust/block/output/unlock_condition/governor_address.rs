// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use iota_types::block::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

/// Defines the Governor Address that owns this output, that is, it can unlock it with the proper Unlock in a
/// transaction that governance transitions the alias output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernorAddressUnlockCondition {
    /// The associated address of this [`GovernorAddressUnlockCondition`].
    pub address: Address,
}

impl<T: Borrow<bee::GovernorAddressUnlockCondition>> From<T> for GovernorAddressUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}

impl From<GovernorAddressUnlockCondition> for bee::GovernorAddressUnlockCondition {
    fn from(value: GovernorAddressUnlockCondition) -> Self {
        Self::new(value.address.into())
    }
}

impl From<GovernorAddressUnlockCondition> for bee::dto::GovernorAddressUnlockConditionDto {
    fn from(value: GovernorAddressUnlockCondition) -> Self {
        Self {
            kind: bee::GovernorAddressUnlockCondition::KIND,
            address: value.address.into(),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use super::*;

    impl GovernorAddressUnlockCondition {
        /// Generates a random [`GovernorAddressUnlockCondition`].
        pub fn rand() -> Self {
            Self {
                address: Address::rand_ed25519(),
            }
        }
    }
}
