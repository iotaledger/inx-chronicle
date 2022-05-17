// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{mem::size_of, str::FromStr};

use bee_message_stardust::output as bee;
use primitive_types::U256;
use serde::{Deserialize, Serialize};

pub type TokenTag = Box<[u8]>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TokenAmount(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<&U256> for TokenAmount {
    fn from(value: &U256) -> Self {
        let mut amount = vec![0; size_of::<U256>()];
        value.to_little_endian(&mut amount);
        Self(amount.into_boxed_slice())
    }
}

impl From<TokenAmount> for U256 {
    fn from(value: TokenAmount) -> Self {
        U256::from_little_endian(value.0.as_ref())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TokenId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<bee::TokenId> for TokenId {
    fn from(value: bee::TokenId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<TokenId> for bee::TokenId {
    type Error = crate::types::error::Error;

    fn try_from(value: TokenId) -> Result<Self, Self::Error> {
        Ok(bee::TokenId::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for TokenId {
    type Err = crate::types::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::TokenId::from_str(s)?.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TokenScheme {
    #[serde(rename = "simple")]
    Simple {
        minted_tokens: TokenAmount,
        melted_tokens: TokenAmount,
        maximum_supply: TokenAmount,
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
    type Error = crate::types::error::Error;

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
    pub token_id: TokenId,
    pub amount: TokenAmount,
}

impl From<&bee::NativeToken> for NativeToken {
    fn from(value: &bee::NativeToken) -> Self {
        Self {
            token_id: TokenId(value.token_id().to_vec().into_boxed_slice()),
            amount: value.amount().into(),
        }
    }
}

impl TryFrom<NativeToken> for bee::NativeToken {
    type Error = crate::types::error::Error;

    fn try_from(value: NativeToken) -> Result<Self, Self::Error> {
        Ok(Self::new(value.token_id.try_into()?, value.amount.into())?)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_token_id_bson() {
        let token_id = TokenId::from(rand_token_id());
        let bson = to_bson(&token_id).unwrap();
        assert_eq!(token_id, from_bson::<TokenId>(bson).unwrap());
    }

    #[test]
    fn test_native_token_bson() {
        let native_token = get_test_native_token();
        let bson = to_bson(&native_token).unwrap();
        assert_eq!(native_token, from_bson::<NativeToken>(bson).unwrap());
    }

    pub(crate) fn rand_token_id() -> bee::TokenId {
        bee_test::rand::bytes::rand_bytes_array().into()
    }

    pub(crate) fn get_test_native_token() -> NativeToken {
        NativeToken::from(&bee::NativeToken::new(bee_test::rand::bytes::rand_bytes_array().into(), 100.into()).unwrap())
    }
}
