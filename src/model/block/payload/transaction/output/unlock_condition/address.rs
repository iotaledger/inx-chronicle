// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::model::Address;

/// Defines the Address that owns an output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressUnlockCondition {
    /// The associated address of this [`AddressUnlockCondition`].
    pub address: Address,
}

impl<T: Borrow<iota::AddressUnlockCondition>> From<T> for AddressUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}

impl From<AddressUnlockCondition> for iota::AddressUnlockCondition {
    fn from(value: AddressUnlockCondition) -> Self {
        Self::new(value.address.into())
    }
}

impl From<AddressUnlockCondition> for iota::dto::AddressUnlockConditionDto {
    fn from(value: AddressUnlockCondition) -> Self {
        Self {
            kind: iota::AddressUnlockCondition::KIND,
            address: value.address.into(),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use super::*;

    impl AddressUnlockCondition {
        /// Generates a random [`AddressUnlockCondition`].
        pub fn rand() -> Self {
            Self {
                address: Address::rand_ed25519(),
            }
        }
    }
}
