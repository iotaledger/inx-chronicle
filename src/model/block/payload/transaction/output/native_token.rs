// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing [`NativeToken`] types.

use std::borrow::Borrow;

use iota_sdk::types::block::output::{self as iota, TokenId};
use primitive_types::U256;
use serde::{Deserialize, Serialize};

/// Defines information about the underlying token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TokenSchemeDto {
    /// The simple token scheme.
    Simple {
        /// The amount of minted (created) tokens.
        minted_tokens: U256,
        /// The amount of melted (destroyed) tokens.
        melted_tokens: U256,
        /// The maximum amount of tokens.
        maximum_supply: U256,
    },
}

impl<T: Borrow<iota::TokenScheme>> From<T> for TokenSchemeDto {
    fn from(value: T) -> Self {
        match value.borrow() {
            iota::TokenScheme::Simple(a) => Self::Simple {
                minted_tokens: a.minted_tokens(),
                melted_tokens: a.melted_tokens(),
                maximum_supply: a.maximum_supply(),
            },
        }
    }
}

impl TryFrom<TokenSchemeDto> for iota::TokenScheme {
    type Error = iota_sdk::types::block::Error;

    fn try_from(value: TokenSchemeDto) -> Result<Self, Self::Error> {
        Ok(match value {
            TokenSchemeDto::Simple {
                minted_tokens,
                melted_tokens,
                maximum_supply,
            } => iota::TokenScheme::Simple(iota::SimpleTokenScheme::new(
                minted_tokens,
                melted_tokens,
                maximum_supply,
            )?),
        })
    }
}

/// Represents a native token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeTokenDto {
    /// The corresponding token id.
    pub token_id: TokenId,
    /// The amount of native tokens.
    pub amount: U256,
}

impl<T: Borrow<iota::NativeToken>> From<T> for NativeTokenDto {
    fn from(value: T) -> Self {
        Self {
            token_id: *value.borrow().token_id(),
            amount: value.borrow().amount(),
        }
    }
}

impl TryFrom<NativeTokenDto> for iota::NativeToken {
    type Error = iota_sdk::types::block::Error;

    fn try_from(value: NativeTokenDto) -> Result<Self, Self::Error> {
        Self::new(value.token_id.into(), value.amount)
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_token_id_bson() {
//         let token_id = NativeTokenId::rand();
//         let bson = to_bson(&token_id).unwrap();
//         assert_eq!(token_id, from_bson::<NativeTokenId>(bson).unwrap());
//     }

//     #[test]
//     fn test_native_token_bson() {
//         let native_token = NativeToken::rand();
//         let bson = to_bson(&native_token).unwrap();
//         assert_eq!(native_token, from_bson::<NativeToken>(bson).unwrap());
//     }

//     #[test]
//     fn test_token_scheme_bson() {
//         let scheme = TokenScheme::rand();
//         let bson = to_bson(&scheme).unwrap();
//         assert_eq!(scheme, from_bson::<TokenScheme>(bson).unwrap());
//     }
// }
