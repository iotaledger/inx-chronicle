// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::address as bee;
use serde::{Deserialize, Serialize};

mod alias;
mod ed25519;
mod nft;

pub use self::{alias::AliasAddress, ed25519::Ed25519Address, nft::NftAddress};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Address {
    #[serde(rename = "ed25519")]
    Ed25519(Ed25519Address),
    #[serde(rename = "alias")]
    Alias(AliasAddress),
    #[serde(rename = "nft")]
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

impl From<Address> for bee::Address {
    fn from(value: Address) -> Self {
        match value {
            Address::Ed25519(a) => Self::Ed25519(a.into()),
            Address::Alias(a) => Self::Alias(a.into()),
            Address::Nft(a) => Self::Nft(a.into()),
        }
    }
}

impl FromStr for Address {
    type Err = crate::db::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::Address::try_from_bech32(s)?.1.into())
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_address_bson() {
        let address = Address::from(bee::Address::Ed25519(bee_test::rand::address::rand_ed25519_address()));
        let bson = to_bson(&address).unwrap();
        assert_eq!(address, from_bson::<Address>(bson).unwrap());

        let address = Address::from(bee::Address::Alias(bee_test::rand::address::rand_alias_address()));
        let bson = to_bson(&address).unwrap();
        assert_eq!(address, from_bson::<Address>(bson).unwrap());

        let address = Address::from(bee::Address::Nft(bee_test::rand::address::rand_nft_address()));
        let bson = to_bson(&address).unwrap();
        assert_eq!(address, from_bson::<Address>(bson).unwrap());
    }
}
