// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::address as stardust;
use serde::{Deserialize, Serialize};

use super::{AliasId, NftId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Address {
    Ed25519(Box<[u8]>),
    Alias(AliasId),
    Nft(NftId),
}

impl From<&stardust::Address> for Address {
    fn from(value: &stardust::Address) -> Self {
        match value {
            stardust::Address::Ed25519(a) => Self::Ed25519(a.to_vec().into_boxed_slice()),
            stardust::Address::Alias(a) => Self::Alias((*a.alias_id()).into()),
            stardust::Address::Nft(a) => Self::Nft((*a.nft_id()).into()),
        }
    }
}

impl TryFrom<Address> for stardust::Address {
    type Error = crate::dto::error::Error;

    fn try_from(value: Address) -> Result<Self, Self::Error> {
        Ok(match value {
            Address::Ed25519(a) => Self::Ed25519(stardust::Ed25519Address::new(a.as_ref().try_into()?)),
            Address::Alias(a) => Self::Alias(stardust::AliasAddress::new(a.try_into()?)),
            Address::Nft(a) => Self::Nft(stardust::NftAddress::new(a.try_into()?)),
        })
    }
}
