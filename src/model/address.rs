// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Address`] types.

use core::borrow::Borrow;

use iota_sdk::types::block::{
    address::{
        self as iota, AddressCapabilities, Ed25519Address, ImplicitAccountCreationAddress, MultiAddress,
        RestrictedAddress,
    },
    output::{AccountId, AnchorId, NftId},
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
        /// The inner address.
        address: CoreAddressDto,
        /// The allowed capabilities bit flags.
        allowed_capabilities: AddressCapabilities,
    },
    /// Multiple addresses with weights.
    Multi(MultiAddressDto),
}

/// The different [`Address`] types supported by restricted addresses.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CoreAddressDto {
    /// An Ed25519 address.
    Ed25519(Ed25519Address),
    /// An account address.
    Account(AccountId),
    /// An NFT address.
    Nft(NftId),
    /// An anchor address.
    Anchor(AnchorId),
}

/// An address with an assigned weight.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct WeightedAddressDto {
    /// The unlocked address.
    address: CoreAddressDto,
    /// The weight of the unlocked address.
    weight: u8,
}

/// An address that consists of addresses with weights and a threshold value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct MultiAddressDto {
    /// The weighted unlocked addresses.
    addresses: Vec<WeightedAddressDto>,
    /// The threshold that needs to be reached by the unlocked addresses in order to unlock the multi address.
    threshold: u16,
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
                    iota::Address::Ed25519(a) => CoreAddressDto::Ed25519(a.clone()),
                    iota::Address::Account(a) => CoreAddressDto::Account(a.into_account_id()),
                    iota::Address::Nft(a) => CoreAddressDto::Nft(a.into_nft_id()),
                    iota::Address::Anchor(a) => CoreAddressDto::Anchor(a.into_anchor_id()),
                    _ => unreachable!(),
                },
                allowed_capabilities: a.allowed_capabilities().clone(),
            },
            iota::Address::Multi(a) => Self::Multi(MultiAddressDto {
                addresses: a
                    .addresses()
                    .iter()
                    .map(|a| WeightedAddressDto {
                        address: match a.address() {
                            iota::Address::Ed25519(a) => CoreAddressDto::Ed25519(a.clone()),
                            iota::Address::Account(a) => CoreAddressDto::Account(a.into_account_id()),
                            iota::Address::Nft(a) => CoreAddressDto::Nft(a.into_nft_id()),
                            iota::Address::Anchor(a) => CoreAddressDto::Anchor(a.into_anchor_id()),
                            _ => unreachable!(),
                        },
                        weight: a.weight(),
                    })
                    .collect(),
                threshold: a.threshold(),
            }),
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
                RestrictedAddress::new(match address {
                    CoreAddressDto::Ed25519(a) => Self::Ed25519(a),
                    CoreAddressDto::Account(a) => Self::Account(a.into()),
                    CoreAddressDto::Nft(a) => Self::Nft(a.into()),
                    CoreAddressDto::Anchor(a) => Self::Anchor(a.into()),
                })
                .unwrap()
                .with_allowed_capabilities(allowed_capabilities),
            )),
            AddressDto::Multi(a) => Self::Multi(
                MultiAddress::new(
                    a.addresses.into_iter().map(|a| {
                        todo!()
                        // WeightedAddress::new(
                        //     match address {
                        //         CoreAddressDto::Ed25519(a) => Self::Ed25519(a),
                        //         CoreAddressDto::Account(a) => Self::Account(a.into()),
                        //         CoreAddressDto::Nft(a) => Self::Nft(a.into()),
                        //         CoreAddressDto::Anchor(a) => Self::Anchor(a.into()),
                        //     },
                        //     a.weight,
                        // )
                    }),
                    a.threshold,
                )
                .unwrap(),
            ),
        }
    }
}

impl From<AddressDto> for Bson {
    fn from(val: AddressDto) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&val).unwrap()
    }
}

#[cfg(test)]
mod test {
    use iota_sdk::types::block::{
        address::Address,
        rand::address::{rand_account_address, rand_anchor_address, rand_ed25519_address, rand_nft_address},
    };
    use mongodb::bson::from_bson;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::model::SerializeToBson;

    #[test]
    fn test_ed25519_address_bson() {
        let address = AddressDto::from(Address::from(rand_ed25519_address()));
        let bson = address.to_bson();
        assert_eq!(Bson::from(address.clone()), bson);
        assert_eq!(address, from_bson::<AddressDto>(bson).unwrap());
    }

    #[test]
    fn test_account_address_bson() {
        let address = AddressDto::from(Address::from(rand_account_address()));
        let bson = address.to_bson();
        assert_eq!(Bson::from(address.clone()), bson);
        assert_eq!(address, from_bson::<AddressDto>(bson).unwrap());
    }

    #[test]
    fn test_nft_address_bson() {
        let address = AddressDto::from(Address::from(rand_nft_address()));
        let bson = address.to_bson();
        assert_eq!(Bson::from(address.clone()), bson);
        assert_eq!(address, from_bson::<AddressDto>(bson).unwrap());
    }

    #[test]
    fn test_anchor_address_bson() {
        let address = AddressDto::from(Address::from(rand_anchor_address()));
        let bson = address.to_bson();
        assert_eq!(Bson::from(address.clone()), bson);
        assert_eq!(address, from_bson::<AddressDto>(bson).unwrap());
    }
}
