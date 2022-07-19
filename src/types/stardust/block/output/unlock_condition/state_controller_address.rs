// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateControllerAddressUnlockCondition {
    pub address: Address,
}

impl From<&bee::StateControllerAddressUnlockCondition> for StateControllerAddressUnlockCondition {
    fn from(value: &bee::StateControllerAddressUnlockCondition) -> Self {
        Self {
            address: value.address().into(),
        }
    }
}

impl From<StateControllerAddressUnlockCondition> for bee::StateControllerAddressUnlockCondition {
    fn from(value: StateControllerAddressUnlockCondition) -> Self {
        Self::new(value.address.into())
    }
}
