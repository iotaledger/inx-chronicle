// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use std::borrow::Borrow;

use iota_types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::model::utxo::Address;

/// Defines the permanent alias address that owns this output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImmutableAliasAddressUnlockCondition {
    /// The associated address of this [`ImmutableAliasAddressUnlockCondition`].
    pub address: Address,
}

impl<T: Borrow<iota::ImmutableAliasAddressUnlockCondition>> From<T> for ImmutableAliasAddressUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}

impl TryFrom<ImmutableAliasAddressUnlockCondition> for iota::ImmutableAliasAddressUnlockCondition {
    type Error = iota_types::block::Error;

    fn try_from(value: ImmutableAliasAddressUnlockCondition) -> Result<Self, Self::Error> {
        use iota_types::block::address::Address as IotaAddress;
        let address = IotaAddress::from(value.address);
        match address {
            IotaAddress::Alias(alias) => Ok(Self::new(alias)),
            other @ (IotaAddress::Ed25519(_) | IotaAddress::Nft(_)) => {
                Err(Self::Error::InvalidAddressKind(other.kind()))
            }
        }
    }
}

impl From<ImmutableAliasAddressUnlockCondition> for iota::dto::ImmutableAliasAddressUnlockConditionDto {
    fn from(value: ImmutableAliasAddressUnlockCondition) -> Self {
        Self {
            kind: iota::ImmutableAliasAddressUnlockCondition::KIND,
            address: value.address.into(),
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
