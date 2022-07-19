// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressUnlockCondition {
    pub address: Address,
}

impl From<&bee::AddressUnlockCondition> for AddressUnlockCondition {
    fn from(value: &bee::AddressUnlockCondition) -> Self {
        Self {
            address: value.address().into(),
        }
    }
}

impl From<AddressUnlockCondition> for bee::AddressUnlockCondition {
    fn from(value: AddressUnlockCondition) -> Self {
        Self::new(value.address.into())
    }
}
