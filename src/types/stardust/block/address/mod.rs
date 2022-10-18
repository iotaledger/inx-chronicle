// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Address`] types.

use std::str::FromStr;

use iota_types::block::address as bee;
use mongodb::bson::{doc, Bson};
use serde::{Deserialize, Serialize};

mod alias;
mod ed25519;
mod nft;

pub use self::{alias::AliasAddress, ed25519::Ed25519Address, nft::NftAddress};

/// The different [`Address`] types supported by the network.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Address {
    /// An Ed25519 address.
    Ed25519(Ed25519Address),
    /// An Alias address.
    Alias(AliasAddress),
    /// An Nft address.
    Nft(NftAddress),
}

impl From<bee::Address> for Address {
    fn from(value: bee::Address) -> Self {
        match value {
            bee::Address::Ed25519(a) => Self::Ed25519(a.into()),
            bee::Address::Alias(a) => Self::Alias(a.into()),
            bee::Address::Nft(a) => Self::Nft(a.into()),
        }
    }
}

impl From<&bee::Address> for Address {
    fn from(value: &bee::Address) -> Self {
        match *value {
            bee::Address::Ed25519(a) => Self::Ed25519(a.into()),
            bee::Address::Alias(a) => Self::Alias(a.into()),
            bee::Address::Nft(a) => Self::Nft(a.into()),
        }
    }
}

impl From<Address> for bee::Address {
    fn from(value: Address) -> Self {
        match value {
            Address::Ed25519(a) => Self::Ed25519(a.into()),
            Address::Alias(a) => Self::Alias(a.into()),
            Address::Nft(a) => Self::Nft(a.into()),
        }
    }
}

impl From<Address> for bee::dto::AddressDto {
    fn from(value: Address) -> Self {
        match value {
            Address::Ed25519(a) => Self::Ed25519(a.into()),
            Address::Alias(a) => Self::Alias(a.into()),
            Address::Nft(a) => Self::Nft(a.into()),
        }
    }
}

impl FromStr for Address {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::Address::try_from_bech32(s)?.1.into())
    }
}

impl From<Address> for Bson {
    fn from(val: Address) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&val).unwrap()
    }
}

#[cfg(feature = "rand")]
mod rand {
    use super::*;

    impl Address {
        /// Generates a random alias [`Address`].
        pub fn rand_alias() -> Self {
            Self::Alias(AliasAddress::rand())
        }

        /// Generates a random nft [`Address`].
        pub fn rand_nft() -> Self {
            Self::Nft(NftAddress::rand())
        }

        /// Generates a ed25519 [`Address`].
        pub fn rand_ed25519() -> Self {
            Self::Ed25519(Ed25519Address::rand())
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_ed25519_address_bson() {
        let address = Address::rand_ed25519();
        let bson = to_bson(&address).unwrap();
        assert_eq!(Bson::from(address), bson);
        assert_eq!(address, from_bson::<Address>(bson).unwrap());
    }

    #[test]
    fn test_alias_address_bson() {
        let address = Address::rand_alias();
        let bson = to_bson(&address).unwrap();
        assert_eq!(Bson::from(address), bson);
        assert_eq!(address, from_bson::<Address>(bson).unwrap());
    }

    #[test]
    fn test_nft_address_bson() {
        let address = Address::rand_nft();
        let bson = to_bson(&address).unwrap();
        assert_eq!(Bson::from(address), bson);
        assert_eq!(address, from_bson::<Address>(bson).unwrap());
    }
}
