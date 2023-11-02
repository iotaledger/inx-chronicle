// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Address`] types.

use core::borrow::Borrow;

use iota_sdk::{
    types::block::{
        address::{self as iota, Ed25519Address, ImplicitAccountCreationAddress, RestrictedAddress},
        output::{AccountId, AnchorId, NftId},
    },
    utils::serde::prefix_hex_bytes,
};
use mongodb::bson::{doc, Bson};
use serde::{Deserialize, Serialize};

/// The different [`Address`] types supported by the network.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AddressDto {
    /// An Ed25519 address.
    Ed25519(Ed25519Address),
    /// An account address.
    Account(AccountId),
    /// An NFT address.
    Nft(NftId),
    /// An anchor address.
    Anchor(AnchorId),
    /// An implicit account creation address.
    ImplicitAccountCreation(ImplicitAccountCreationAddress),
    /// An address with restricted capabilities.
    Restricted {
        address: RestrictedAddressDto,
        // TODO: Use the real type
        #[serde(with = "prefix_hex_bytes")]
        allowed_capabilities: Box<[u8]>,
    },
}

/// The different [`Address`] types supported by restricted addresses.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RestrictedAddressDto {
    /// An Ed25519 address.
    Ed25519(Ed25519Address),
    /// An account address.
    Account(AccountId),
    /// An NFT address.
    Nft(NftId),
    /// An anchor address.
    Anchor(AnchorId),
}

impl<T: Borrow<iota::Address>> From<T> for AddressDto {
    fn from(value: T) -> Self {
        match value.borrow() {
            iota::Address::Ed25519(a) => Self::Ed25519(a.clone()),
            iota::Address::Account(a) => Self::Account(a.into_account_id()),
            iota::Address::Nft(a) => Self::Nft(a.into_nft_id()),
            iota::Address::Anchor(a) => Self::Anchor(a.into_anchor_id()),
            iota::Address::ImplicitAccountCreation(a) => Self::ImplicitAccountCreation(a.clone()),
            iota::Address::Restricted(a) => Self::Restricted {
                address: match a.address() {
                    iota::Address::Ed25519(a) => RestrictedAddressDto::Ed25519(a.clone()),
                    iota::Address::Account(a) => RestrictedAddressDto::Account(a.into_account_id()),
                    iota::Address::Nft(a) => RestrictedAddressDto::Nft(a.into_nft_id()),
                    iota::Address::Anchor(a) => RestrictedAddressDto::Anchor(a.into_anchor_id()),
                    _ => unreachable!(),
                },
                allowed_capabilities: a.allowed_capabilities().iter().copied().collect(),
            },
        }
    }
}

impl From<AddressDto> for iota::Address {
    fn from(value: AddressDto) -> Self {
        match value {
            AddressDto::Ed25519(a) => Self::Ed25519(a),
            AddressDto::Account(a) => Self::Account(a.into()),
            AddressDto::Nft(a) => Self::Nft(a.into()),
            AddressDto::Anchor(a) => Self::Anchor(a.into()),
            AddressDto::ImplicitAccountCreation(a) => Self::ImplicitAccountCreation(a),
            AddressDto::Restricted {
                address,
                allowed_capabilities,
            } => Self::Restricted(Box::new(
                // TODO: address capabilities
                RestrictedAddress::new(match address {
                    RestrictedAddressDto::Ed25519(a) => Self::Ed25519(a),
                    RestrictedAddressDto::Account(a) => Self::Account(a.into()),
                    RestrictedAddressDto::Nft(a) => Self::Nft(a.into()),
                    RestrictedAddressDto::Anchor(a) => Self::Anchor(a.into()),
                })
                .unwrap(),
            )),
        }
    }
}

impl From<AddressDto> for Bson {
    fn from(val: AddressDto) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&val).unwrap()
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_ed25519_address_bson() {
//         let address = AddressDto::rand_ed25519();
//         let bson = to_bson(&address).unwrap();
//         assert_eq!(Bson::from(address), bson);
//         assert_eq!(address, from_bson::<AddressDto>(bson).unwrap());
//     }

//     #[test]
//     fn test_alias_address_bson() {
//         let address = AddressDto::rand_alias();
//         let bson = to_bson(&address).unwrap();
//         assert_eq!(Bson::from(address), bson);
//         assert_eq!(address, from_bson::<AddressDto>(bson).unwrap());
//     }

//     #[test]
//     fn test_nft_address_bson() {
//         let address = AddressDto::rand_nft();
//         let bson = to_bson(&address).unwrap();
//         assert_eq!(Bson::from(address), bson);
//         assert_eq!(address, from_bson::<AddressDto>(bson).unwrap());
//     }
// }
