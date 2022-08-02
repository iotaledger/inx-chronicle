// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{mem::size_of, str::FromStr};

use bee_block_stardust::output as bee;
use primitive_types::U256;
use serde::{Deserialize, Serialize};

use crate::types::util::bytify;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NativeTokenAmount(#[serde(with = "bytify")] pub [u8; size_of::<U256>()]);

impl From<&U256> for NativeTokenAmount {
    fn from(value: &U256) -> Self {
        let mut amount = [0; size_of::<U256>()];
        value.to_big_endian(&mut amount);
        Self(amount)
    }
}

impl From<NativeTokenAmount> for U256 {
    fn from(value: NativeTokenAmount) -> Self {
        U256::from_big_endian(&value.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NativeTokenId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl NativeTokenId {
    const LENGTH: usize = bee::TokenId::LENGTH;
}

impl From<bee::TokenId> for NativeTokenId {
    fn from(value: bee::TokenId) -> Self {
        Self(*value)
    }
}

impl From<NativeTokenId> for bee::TokenId {
    fn from(value: NativeTokenId) -> Self {
        bee::TokenId::new(value.0)
    }
}

impl FromStr for NativeTokenId {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::TokenId::from_str(s)?.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TokenScheme {
    Simple {
        minted_tokens: NativeTokenAmount,
        melted_tokens: NativeTokenAmount,
        maximum_supply: NativeTokenAmount,
    },
}

impl From<&bee::TokenScheme> for TokenScheme {
    fn from(value: &bee::TokenScheme) -> Self {
        match value {
            bee::TokenScheme::Simple(a) => Self::Simple {
                minted_tokens: a.minted_tokens().into(),
                melted_tokens: a.melted_tokens().into(),
                maximum_supply: a.maximum_supply().into(),
            },
        }
    }
}

impl TryFrom<TokenScheme> for bee::TokenScheme {
    type Error = bee_block_stardust::Error;

    fn try_from(value: TokenScheme) -> Result<Self, Self::Error> {
        Ok(match value {
            TokenScheme::Simple {
                minted_tokens,
                melted_tokens,
                maximum_supply,
            } => bee::TokenScheme::Simple(bee::SimpleTokenScheme::new(
                minted_tokens.into(),
                melted_tokens.into(),
                maximum_supply.into(),
            )?),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeToken {
    pub token_id: NativeTokenId,
    pub amount: NativeTokenAmount,
}

impl From<&bee::NativeToken> for NativeToken {
    fn from(value: &bee::NativeToken) -> Self {
        Self {
            token_id: NativeTokenId(**value.token_id()),
            amount: value.amount().into(),
        }
    }
}

impl TryFrom<NativeToken> for bee::NativeToken {
    type Error = bee_block_stardust::Error;

    fn try_from(value: NativeToken) -> Result<Self, Self::Error> {
        Self::new(value.token_id.into(), value.amount.into())
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_token_id_bson() {
        let token_id = NativeTokenId::from(rand_token_id());
        let bson = to_bson(&token_id).unwrap();
        assert_eq!(token_id, from_bson::<NativeTokenId>(bson).unwrap());
    }

    #[test]
    fn test_native_token_bson() {
        let native_token = get_test_native_token();
        let bson = to_bson(&native_token).unwrap();
        assert_eq!(native_token, from_bson::<NativeToken>(bson).unwrap());
    }

    pub(crate) fn rand_token_id() -> bee::TokenId {
        bee_block_stardust::rand::bytes::rand_bytes_array().into()
    }

    pub(crate) fn get_test_native_token() -> NativeToken {
        NativeToken::from(
            &bee::NativeToken::new(bee_block_stardust::rand::bytes::rand_bytes_array().into(), 100.into()).unwrap(),
        )
    }
}
