// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernorAddressUnlockCondition {
    pub address: Address,
}

impl From<&bee::GovernorAddressUnlockCondition> for GovernorAddressUnlockCondition {
    fn from(value: &bee::GovernorAddressUnlockCondition) -> Self {
        Self {
            address: value.address().into(),
        }
    }
}

impl From<GovernorAddressUnlockCondition> for bee::GovernorAddressUnlockCondition {
    fn from(value: GovernorAddressUnlockCondition) -> Self {
        Self::new(value.address.into())
    }
}
