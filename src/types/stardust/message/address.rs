// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_message_stardust::address as stardust;
use serde::{Deserialize, Serialize};

use super::{AliasId, NftId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ed25519Address(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<stardust::Ed25519Address> for Ed25519Address {
    fn from(value: stardust::Ed25519Address) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<Ed25519Address> for stardust::Ed25519Address {
    type Error = crate::types::error::Error;

    fn try_from(value: Ed25519Address) -> Result<Self, Self::Error> {
        Ok(stardust::Ed25519Address::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for Ed25519Address {
    type Err = crate::types::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(stardust::Ed25519Address::from_str(s)?.into())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Address {
    #[serde(rename = "ed25519")]
    Ed25519(Ed25519Address),
    #[serde(rename = "nft")]
    Alias(AliasId),
    #[serde(rename = "nft")]
    Nft(NftId),
}

impl From<stardust::Address> for Address {
    fn from(value: stardust::Address) -> Self {
        match value {
            stardust::Address::Ed25519(a) => Self::Ed25519(Ed25519Address::from(a)),
            stardust::Address::Alias(a) => Self::Alias((*a.alias_id()).into()),
            stardust::Address::Nft(a) => Self::Nft((*a.nft_id()).into()),
        }
    }
}

impl TryFrom<Address> for stardust::Address {
    type Error = crate::types::error::Error;

    fn try_from(value: Address) -> Result<Self, Self::Error> {
        Ok(match value {
            Address::Ed25519(a) => Self::Ed25519(a.try_into()?),
            Address::Alias(a) => Self::Alias(stardust::AliasAddress::new(a.try_into()?)),
            Address::Nft(a) => Self::Nft(stardust::NftAddress::new(a.try_into()?)),
        })
    }
}

impl FromStr for Address {
    type Err = crate::types::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(stardust::Address::try_from_bech32(s)?.1.into())
    }
}

#[cfg(test)]
pub(crate) mod test {
    pub(crate) const ED25519_ADDRESS: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c649";

    use std::str::FromStr;

    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::message::output::test::OUTPUT_ID;

    #[test]
    fn test_ed25519_bson() {
        let address = Address::Ed25519(Ed25519Address::from_str(ED25519_ADDRESS).unwrap());
        let bson = to_bson(&address).unwrap();
        from_bson::<Address>(bson).unwrap();
    }

    #[test]
    fn test_alias_bson() {
        let address = Address::Alias(AliasId::from_output_id_str(OUTPUT_ID).unwrap());
        let bson = to_bson(&address).unwrap();
        from_bson::<Address>(bson).unwrap();
    }

    #[test]
    fn test_nft_bson() {
        let address = Address::Nft(NftId::from_output_id_str(OUTPUT_ID).unwrap());
        let bson = to_bson(&address).unwrap();
        from_bson::<Address>(bson).unwrap();
    }

    pub(crate) fn get_test_ed25519_address() -> Address {
        Address::Ed25519(Ed25519Address::from_str(ED25519_ADDRESS).unwrap())
    }

    pub(crate) fn get_test_alias_address() -> Address {
        Address::Alias(AliasId::from_output_id_str(OUTPUT_ID).unwrap())
    }

    pub(crate) fn get_test_nft_address() -> Address {
        Address::Nft(NftId::from_output_id_str(OUTPUT_ID).unwrap())
    }
}
