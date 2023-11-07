// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Output`] types.

pub mod account;
pub mod address;
pub mod anchor;
pub mod basic;
pub mod delegation;
pub mod feature;
pub mod foundry;
pub mod native_token;
pub mod nft;
pub mod unlock_condition;

use std::{borrow::Borrow, str::FromStr};

use iota_sdk::types::block::output::{self as iota, Output};
use mongodb::bson::{doc, Bson};
use serde::{Deserialize, Serialize};

pub use self::{
    account::AccountOutputDto,
    address::AddressDto,
    anchor::AnchorOutputDto,
    basic::BasicOutputDto,
    delegation::DelegationOutputDto,
    feature::FeatureDto,
    foundry::FoundryOutputDto,
    native_token::{NativeTokenDto, TokenSchemeDto},
    nft::NftOutputDto,
};
use crate::model::TryFromDto;

/// Represents the different output types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[allow(missing_docs)]
pub enum OutputDto {
    Basic(BasicOutputDto),
    Account(AccountOutputDto),
    Foundry(FoundryOutputDto),
    Nft(NftOutputDto),
    Delegation(DelegationOutputDto),
    Anchor(AnchorOutputDto),
}

impl OutputDto {
    /// Returns the [`Address`] that is in control of the output.
    pub fn owning_address(&self) -> Option<&AddressDto> {
        Some(match self {
            Self::Basic(BasicOutputDto {
                address_unlock_condition,
                ..
            }) => &address_unlock_condition.address,
            Self::Account(AccountOutputDto {
                address_unlock_condition,
                ..
            }) => &address_unlock_condition.address,
            Self::Foundry(FoundryOutputDto {
                immutable_account_address_unlock_condition,
                ..
            }) => &immutable_account_address_unlock_condition.address,
            Self::Nft(NftOutputDto {
                address_unlock_condition,
                ..
            }) => &address_unlock_condition.address,
            Self::Delegation(DelegationOutputDto {
                address_unlock_condition,
                ..
            }) => &address_unlock_condition.address,
            Self::Anchor(AnchorOutputDto {
                state_controller_unlock_condition,
                ..
            }) => &state_controller_unlock_condition.address,
        })
    }

    /// Returns the amount associated with an output.
    pub fn amount(&self) -> u64 {
        match self {
            Self::Basic(BasicOutputDto { amount, .. }) => *amount,
            Self::Account(AccountOutputDto { amount, .. }) => *amount,
            Self::Nft(NftOutputDto { amount, .. }) => *amount,
            Self::Foundry(FoundryOutputDto { amount, .. }) => *amount,
            Self::Delegation(DelegationOutputDto { amount, .. }) => *amount,
            Self::Anchor(AnchorOutputDto { amount, .. }) => *amount,
        }
    }

    /// Checks if an output is trivially unlockable by only providing a signature.
    pub fn is_trivial_unlock(&self) -> bool {
        match self {
            Self::Basic(BasicOutputDto {
                storage_deposit_return_unlock_condition,
                timelock_unlock_condition,
                expiration_unlock_condition,
                ..
            }) => {
                storage_deposit_return_unlock_condition.is_none()
                    && timelock_unlock_condition.is_none()
                    && expiration_unlock_condition.is_none()
            }
            Self::Account(_) => true,
            Self::Nft(NftOutputDto {
                storage_deposit_return_unlock_condition,
                timelock_unlock_condition,
                expiration_unlock_condition,
                ..
            }) => {
                storage_deposit_return_unlock_condition.is_none()
                    && timelock_unlock_condition.is_none()
                    && expiration_unlock_condition.is_none()
            }
            Self::Foundry(_) => true,
            Self::Delegation(_) => true,
            Self::Anchor(_) => true,
        }
    }

    // /// Converts the [`Output`] into its raw byte representation.
    // pub fn raw(self, ctx: &ProtocolParameters) -> Result<Vec<u8>, iota_sdk::types::block::Error> {
    //     let output = iota_sdk::types::block::output::Output::try_from_dto(self, ctx)?;
    //     Ok(output.pack_to_vec())
    // }

    /// Get the output kind as a string.
    pub fn kind(&self) -> &str {
        match self {
            Self::Basic(_) => BasicOutputDto::KIND,
            Self::Account(_) => AccountOutputDto::KIND,
            Self::Foundry(_) => FoundryOutputDto::KIND,
            Self::Nft(_) => NftOutputDto::KIND,
            Self::Delegation(_) => DelegationOutputDto::KIND,
            Self::Anchor(_) => AnchorOutputDto::KIND,
        }
    }
}

impl<T: Borrow<iota::Output>> From<T> for OutputDto {
    fn from(value: T) -> Self {
        match value.borrow() {
            iota::Output::Basic(o) => Self::Basic(o.into()),
            iota::Output::Account(o) => Self::Account(o.into()),
            iota::Output::Foundry(o) => Self::Foundry(o.into()),
            iota::Output::Nft(o) => Self::Nft(o.into()),
            iota::Output::Delegation(o) => Self::Delegation(o.into()),
            iota::Output::Anchor(o) => Self::Anchor(o.into()),
        }
    }
}

impl From<OutputDto> for iota_sdk::types::block::output::dto::OutputDto {
    fn from(value: OutputDto) -> Self {
        match value {
            OutputDto::Basic(b) => Self::Basic(b.into()),
            OutputDto::Account(_) => todo!(),
            OutputDto::Foundry(_) => todo!(),
            OutputDto::Nft(_) => todo!(),
            OutputDto::Delegation(_) => todo!(),
            OutputDto::Anchor(_) => todo!(),
        }
    }
}

impl TryFromDto<OutputDto> for Output {
    type Error = iota_sdk::types::block::Error;

    fn try_from_dto_with_params_inner(
        dto: OutputDto,
        params: iota_sdk::types::ValidationParams<'_>,
    ) -> Result<Self, Self::Error> {
        iota_sdk::types::TryFromDto::try_from_dto(dto.into())
    }
}

/// A [`Tag`] associated with an [`Output`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(#[serde(with = "serde_bytes")] Vec<u8>);

impl Tag {
    /// Creates a [`Tag`] from `0x`-prefixed hex representation.
    pub fn from_hex<T: AsRef<str>>(tag: T) -> Result<Self, prefix_hex::Error> {
        Ok(Self(prefix_hex::decode::<Vec<u8>>(tag.as_ref())?))
    }

    /// Converts the [`Tag`] to its `0x`-prefixed hex representation.
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(&*self.0)
    }
}

// Note: assumes an ASCII string as input.
impl<T: ToString> From<T> for Tag {
    fn from(value: T) -> Self {
        Self(value.to_string().into_bytes())
    }
}

// Note: assumes a `0x`-prefixed hex representation as input.
impl FromStr for Tag {
    type Err = prefix_hex::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

impl From<Tag> for Bson {
    fn from(val: Tag) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&serde_bytes::ByteBuf::from(val.0)).unwrap()
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_output_id_bson() {
//         let output_id = OutputIdDto::rand();
//         let bson = to_bson(&output_id).unwrap();
//         from_bson::<OutputIdDto>(bson).unwrap();
//     }

//     #[test]
//     fn test_basic_output_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let output = OutputDto::rand_basic(&ctx);
//         iota::Output::try_from_with_context(&ctx, output.clone()).unwrap();
//         let bson = to_bson(&output).unwrap();
//         assert_eq!(
//             bson.as_document().unwrap().get_str("kind").unwrap(),
//             BasicOutputDto::KIND
//         );
//         assert_eq!(output, from_bson::<OutputDto>(bson).unwrap());
//     }

//     #[test]
//     fn test_alias_output_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let output = OutputDto::rand_alias(&ctx);
//         iota::Output::try_from_with_context(&ctx, output.clone()).unwrap();
//         let bson = to_bson(&output).unwrap();
//         assert_eq!(
//             bson.as_document().unwrap().get_str("kind").unwrap(),
//             AccountOutputDto::KIND
//         );
//         assert_eq!(output, from_bson::<OutputDto>(bson).unwrap());
//     }

//     #[test]
//     fn test_nft_output_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let output = OutputDto::rand_nft(&ctx);
//         iota::Output::try_from_with_context(&ctx, output.clone()).unwrap();
//         let bson = to_bson(&output).unwrap();
//         assert_eq!(bson.as_document().unwrap().get_str("kind").unwrap(), NftOutputDto::KIND);
//         assert_eq!(output, from_bson::<OutputDto>(bson).unwrap());
//     }

//     #[test]
//     fn test_foundry_output_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let output = OutputDto::rand_foundry(&ctx);
//         iota::Output::try_from_with_context(&ctx, output.clone()).unwrap();
//         let bson = to_bson(&output).unwrap();
//         assert_eq!(
//             bson.as_document().unwrap().get_str("kind").unwrap(),
//             FoundryOutputDto::KIND
//         );
//         assert_eq!(output, from_bson::<OutputDto>(bson).unwrap());
//     }

//     #[test]
//     fn test_treasury_output_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let output = OutputDto::rand_treasury(&ctx);
//         iota::Output::try_from_with_context(&ctx, output.clone()).unwrap();
//         let bson = to_bson(&output).unwrap();
//         assert_eq!(
//             bson.as_document().unwrap().get_str("kind").unwrap(),
//             TreasuryOutputDto::KIND
//         );
//         assert_eq!(output, from_bson::<OutputDto>(bson).unwrap());
//     }
// }
