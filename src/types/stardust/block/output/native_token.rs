// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{borrow::Borrow, mem::size_of, str::FromStr};

use iota_types::block::output as iota;
use primitive_types::U256;
use serde::{Deserialize, Serialize};

use crate::types::util::bytify;

/// Represents the amount of native tokens.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NativeTokenAmount(#[serde(with = "bytify")] pub [u8; size_of::<U256>()]);

impl<T: Borrow<U256>> From<T> for NativeTokenAmount {
    fn from(value: T) -> Self {
        let mut amount = [0; size_of::<U256>()];
        value.borrow().to_big_endian(&mut amount);
        Self(amount)
    }
}

impl From<NativeTokenAmount> for U256 {
    fn from(value: NativeTokenAmount) -> Self {
        U256::from_big_endian(&value.0)
    }
}

impl From<NativeTokenAmount> for iota_types::block::dto::U256Dto {
    fn from(value: NativeTokenAmount) -> Self {
        Into::into(&U256::from(value))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NativeTokenId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl NativeTokenId {
    const LENGTH: usize = iota::TokenId::LENGTH;
}

impl From<iota::TokenId> for NativeTokenId {
    fn from(value: iota::TokenId) -> Self {
        Self(*value)
    }
}

impl From<NativeTokenId> for iota::TokenId {
    fn from(value: NativeTokenId) -> Self {
        iota::TokenId::new(value.0)
    }
}

impl From<NativeTokenId> for iota::dto::TokenIdDto {
    fn from(value: NativeTokenId) -> Self {
        Into::into(&iota::TokenId::from(value))
    }
}

impl FromStr for NativeTokenId {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::TokenId::from_str(s)?.into())
    }
}

/// Defines information about the underlying token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TokenScheme {
    /// The simple token scheme.
    Simple {
        /// The amount of minted (created) tokens.
        minted_tokens: NativeTokenAmount,
        /// The amount of melted (destroyed) tokens.
        melted_tokens: NativeTokenAmount,
        /// The maximum amount of tokens.
        maximum_supply: NativeTokenAmount,
    },
}

impl<T: Borrow<iota::TokenScheme>> From<T> for TokenScheme {
    fn from(value: T) -> Self {
        match value.borrow() {
            iota::TokenScheme::Simple(a) => Self::Simple {
                minted_tokens: a.minted_tokens().into(),
                melted_tokens: a.melted_tokens().into(),
                maximum_supply: a.maximum_supply().into(),
            },
        }
    }
}

impl TryFrom<TokenScheme> for iota::TokenScheme {
    type Error = iota_types::block::Error;

    fn try_from(value: TokenScheme) -> Result<Self, Self::Error> {
        Ok(match value {
            TokenScheme::Simple {
                minted_tokens,
                melted_tokens,
                maximum_supply,
            } => iota::TokenScheme::Simple(iota::SimpleTokenScheme::new(
                minted_tokens.into(),
                melted_tokens.into(),
                maximum_supply.into(),
            )?),
        })
    }
}

impl From<TokenScheme> for iota::dto::TokenSchemeDto {
    fn from(value: TokenScheme) -> Self {
        match value {
            TokenScheme::Simple {
                minted_tokens,
                melted_tokens,
                maximum_supply,
            } => Self::Simple(iota::dto::SimpleTokenSchemeDto {
                kind: iota::SimpleTokenScheme::KIND,
                minted_tokens: minted_tokens.into(),
                melted_tokens: melted_tokens.into(),
                maximum_supply: maximum_supply.into(),
            }),
        }
    }
}

/// Represents a native token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeToken {
    /// The corresponding token id.
    pub token_id: NativeTokenId,
    /// The amount of native tokens.
    pub amount: NativeTokenAmount,
}

impl<T: Borrow<iota::NativeToken>> From<T> for NativeToken {
    fn from(value: T) -> Self {
        Self {
            token_id: NativeTokenId(**value.borrow().token_id()),
            amount: value.borrow().amount().into(),
        }
    }
}

impl TryFrom<NativeToken> for iota::NativeToken {
    type Error = iota_types::block::Error;

    fn try_from(value: NativeToken) -> Result<Self, Self::Error> {
        Self::new(value.token_id.into(), value.amount.into())
    }
}

impl From<NativeToken> for iota::dto::NativeTokenDto {
    fn from(value: NativeToken) -> Self {
        Self {
            token_id: value.token_id.into(),
            amount: value.amount.into(),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::{
        bytes::{rand_bytes, rand_bytes_array},
        output::rand_token_scheme,
    };

    use super::*;

    impl NativeTokenAmount {
        /// Generates a random [`NativeToken`].
        pub fn rand() -> Self {
            U256::from_little_endian(&rand_bytes(32)).max(1.into()).into()
        }
    }

    impl NativeTokenId {
        /// Generates a random [`NativeTokenId`].
        pub fn rand() -> Self {
            Self(rand_bytes_array())
        }
    }

    impl NativeToken {
        /// Generates a random [`NativeToken`].
        pub fn rand() -> Self {
            Self {
                token_id: NativeTokenId::rand(),
                amount: NativeTokenAmount::rand(),
            }
        }

        /// Generates multiple random [`NativeTokens`](NativeToken).
        pub fn rand_many(len: usize) -> impl Iterator<Item = Self> {
            std::iter::repeat_with(NativeToken::rand).take(len)
        }
    }

    impl TokenScheme {
        /// Generates a random [`TokenScheme`].
        pub fn rand() -> Self {
            rand_token_scheme().into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_token_id_bson() {
        let token_id = NativeTokenId::rand();
        let bson = to_bson(&token_id).unwrap();
        assert_eq!(token_id, from_bson::<NativeTokenId>(bson).unwrap());
    }

    #[test]
    fn test_native_token_bson() {
        let native_token = NativeToken::rand();
        let bson = to_bson(&native_token).unwrap();
        assert_eq!(native_token, from_bson::<NativeToken>(bson).unwrap());
    }

    #[test]
    fn test_token_scheme_bson() {
        let scheme = TokenScheme::rand();
        let bson = to_bson(&scheme).unwrap();
        assert_eq!(scheme, from_bson::<TokenScheme>(bson).unwrap());
    }
}
