// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::mem::size_of;

use bee_block_stardust::output as stardust;
use primitive_types::U256;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TokenAmount(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<&U256> for TokenAmount {
    fn from(value: &U256) -> Self {
        let mut amount = Vec::with_capacity(size_of::<U256>());
        value.to_little_endian(&mut amount);
        Self(amount.into_boxed_slice())
    }
}

impl From<TokenAmount> for U256 {
    fn from(value: TokenAmount) -> Self {
        U256::from_little_endian(value.0.as_ref())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TokenId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<stardust::TokenId> for TokenId {
    fn from(value: stardust::TokenId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<TokenId> for stardust::TokenId {
    type Error = crate::types::error::Error;

    fn try_from(value: TokenId) -> Result<Self, Self::Error> {
        Ok(stardust::TokenId::new(value.0.as_ref().try_into()?))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TokenScheme {
    #[serde(rename = "simple")]
    Simple {
        minted_tokens: TokenAmount,
        melted_tokens: TokenAmount,
        maximum_supply: TokenAmount,
    },
}

impl From<&stardust::TokenScheme> for TokenScheme {
    fn from(value: &stardust::TokenScheme) -> Self {
        match value {
            stardust::TokenScheme::Simple(a) => Self::Simple {
                minted_tokens: a.minted_tokens().into(),
                melted_tokens: a.melted_tokens().into(),
                maximum_supply: a.maximum_supply().into(),
            },
        }
    }
}

impl TryFrom<TokenScheme> for stardust::TokenScheme {
    type Error = crate::types::error::Error;

    fn try_from(value: TokenScheme) -> Result<Self, Self::Error> {
        Ok(match value {
            TokenScheme::Simple {
                minted_tokens,
                melted_tokens,
                maximum_supply,
            } => stardust::TokenScheme::Simple(stardust::SimpleTokenScheme::new(
                minted_tokens.into(),
                melted_tokens.into(),
                maximum_supply.into(),
            )?),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NativeToken {
    pub token_id: TokenId,
    pub amount: TokenAmount,
}

impl From<&stardust::NativeToken> for NativeToken {
    fn from(value: &stardust::NativeToken) -> Self {
        Self {
            token_id: TokenId(value.token_id().to_vec().into_boxed_slice()),
            amount: value.amount().into(),
        }
    }
}

impl TryFrom<NativeToken> for stardust::NativeToken {
    type Error = crate::types::error::Error;

    fn try_from(value: NativeToken) -> Result<Self, Self::Error> {
        Ok(Self::new(value.token_id.try_into()?, value.amount.into())?)
    }
}
