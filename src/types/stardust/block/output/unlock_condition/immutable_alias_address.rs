use std::borrow::Borrow;

use iota_types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

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
        let address = iota_types::block::address::Address::from(value.address);
        match address {
            iota_types::block::address::Address::Alias(alias) => Ok(Self::new(alias)),
            other @ (iota_types::block::address::Address::Ed25519(_) | iota_types::block::address::Address::Nft(_)) => {
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
