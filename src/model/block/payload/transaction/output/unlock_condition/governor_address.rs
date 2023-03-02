// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::model::utxo::Address;

/// Defines the Governor Address that owns this output, that is, it can unlock it with the proper Unlock in a
/// transaction that governance transitions the alias output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernorAddressUnlockCondition {
    /// The associated address of this [`GovernorAddressUnlockCondition`].
    pub address: Address,
}

impl<T: Borrow<iota::GovernorAddressUnlockCondition>> From<T> for GovernorAddressUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}

impl From<GovernorAddressUnlockCondition> for iota::GovernorAddressUnlockCondition {
    fn from(value: GovernorAddressUnlockCondition) -> Self {
        Self::new(value.address.into())
    }
}

impl From<GovernorAddressUnlockCondition> for iota::dto::GovernorAddressUnlockConditionDto {
    fn from(value: GovernorAddressUnlockCondition) -> Self {
        Self {
            kind: iota::GovernorAddressUnlockCondition::KIND,
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
