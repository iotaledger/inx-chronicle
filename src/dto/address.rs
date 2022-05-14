// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::address as stardust;
use serde::{Deserialize, Serialize};

use super::{AliasId, NftId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ed25519Address(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<&stardust::Ed25519Address> for Ed25519Address {
    fn from(value: &stardust::Ed25519Address) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<Ed25519Address> for stardust::Ed25519Address {
    type Error = crate::dto::error::Error;

    fn try_from(value: Ed25519Address) -> Result<Self, Self::Error> {
        Ok(stardust::Ed25519Address::new(value.0.as_ref().try_into()?))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Address {
    #[serde(rename = "ed25519")]
    Ed25519(Ed25519Address),
    #[serde(rename = "alias")]
    Alias(AliasId),
    #[serde(rename = "nft")]
    Nft(NftId),
}

impl From<&stardust::Address> for Address {
    fn from(value: &stardust::Address) -> Self {
        match value {
            stardust::Address::Ed25519(a) => Self::Ed25519(Ed25519Address::from(a)),
            stardust::Address::Alias(a) => Self::Alias((*a.alias_id()).into()),
            stardust::Address::Nft(a) => Self::Nft((*a.nft_id()).into()),
        }
    }
}

impl TryFrom<Address> for stardust::Address {
    type Error = crate::dto::error::Error;

    fn try_from(value: Address) -> Result<Self, Self::Error> {
        Ok(match value {
            Address::Ed25519(a) => Self::Ed25519(a.try_into()?),
            Address::Alias(a) => Self::Alias(stardust::AliasAddress::new(a.try_into()?)),
            Address::Nft(a) => Self::Nft(stardust::NftAddress::new(a.try_into()?)),
        })
    }
}
