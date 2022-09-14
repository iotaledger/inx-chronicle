// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::output::unlock_condition as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImmutableAliasAddressUnlockCondition {
    pub address: Address,
}

impl<T: Borrow<bee::ImmutableAliasAddressUnlockCondition>> From<T> for ImmutableAliasAddressUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}

impl TryFrom<ImmutableAliasAddressUnlockCondition> for bee::ImmutableAliasAddressUnlockCondition {
    type Error = bee_block_stardust::Error;

    fn try_from(value: ImmutableAliasAddressUnlockCondition) -> Result<Self, Self::Error> {
        let address = bee_block_stardust::address::Address::from(value.address);
        match address {
            bee_block_stardust::address::Address::Alias(alias) => Ok(Self::new(alias)),
            other @ (bee_block_stardust::address::Address::Ed25519(_)
            | bee_block_stardust::address::Address::Nft(_)) => Err(Self::Error::InvalidAddressKind(other.kind())),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use super::*;

    impl ImmutableAliasAddressUnlockCondition {
        /// Generates a random [`ImmutableAliasAddressUnlockCondition`].
        pub fn rand() -> Self {
            Self {
                address: Address::rand_alias(),
            }
        }
    }
}
