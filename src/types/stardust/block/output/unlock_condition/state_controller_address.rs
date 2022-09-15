// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateControllerAddressUnlockCondition {
    pub address: Address,
}

impl<T: Borrow<bee::StateControllerAddressUnlockCondition>> From<T> for StateControllerAddressUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}

impl From<StateControllerAddressUnlockCondition> for bee::StateControllerAddressUnlockCondition {
    fn from(value: StateControllerAddressUnlockCondition) -> Self {
        Self::new(value.address.into())
    }
}

#[cfg(feature = "rand")]
mod rand {
    use super::*;

    impl StateControllerAddressUnlockCondition {
        /// Generates a random [`StateControllerAddressUnlockCondition`].
        pub fn rand() -> Self {
            Self {
                address: Address::rand_ed25519(),
            }
        }
    }
}
