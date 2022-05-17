// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_message_stardust::address as bee;
use serde::{Deserialize, Serialize};

use super::{AliasId, NftId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ed25519Address(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<bee::Ed25519Address> for Ed25519Address {
    fn from(value: bee::Ed25519Address) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<Ed25519Address> for bee::Ed25519Address {
    type Error = crate::types::error::Error;

    fn try_from(value: Ed25519Address) -> Result<Self, Self::Error> {
        Ok(bee::Ed25519Address::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for Ed25519Address {
    type Err = crate::types::error::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::Ed25519Address::from_str(s)?.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Address {
    #[serde(rename = "ed25519")]
    Ed25519(Ed25519Address),
    #[serde(rename = "alias")]
    Alias(AliasId),
    #[serde(rename = "nft")]
    Nft(NftId),
}

impl From<bee::Address> for Address {
    fn from(value: bee::Address) -> Self {
        match value {
            bee::Address::Ed25519(a) => Self::Ed25519(Ed25519Address::from(a)),
            bee::Address::Alias(a) => Self::Alias((*a.alias_id()).into()),
            bee::Address::Nft(a) => Self::Nft((*a.nft_id()).into()),
        }
    }
}

impl TryFrom<Address> for bee::Address {
    type Error = crate::types::error::Error;

    fn try_from(value: Address) -> Result<Self, Self::Error> {
        Ok(match value {
            Address::Ed25519(a) => Self::Ed25519(a.try_into()?),
            Address::Alias(a) => Self::Alias(bee::AliasAddress::new(a.try_into()?)),
            Address::Nft(a) => Self::Nft(bee::NftAddress::new(a.try_into()?)),
        })
    }
}

impl FromStr for Address {
    type Err = crate::types::error::ParseError;

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
