// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::mem::size_of;

use serde::{Deserialize, Serialize};
use bee_message_stardust::output as stardust;
use primitive_types::U256;

pub type TokenAmount = Box<[u8]>;
pub type TokenId = Box<[u8]>;
pub type TokenTag = Box<[u8]>;

use crate::dto::error::Error;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum TokenScheme {
    SimpleTokenScheme {
        minted_tokens: TokenAmount,
        melted_tokens: TokenAmount,
        maximum_supply: TokenAmount,
    },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct NativeToken {
    pub token_id: TokenId,
    pub amount: TokenAmount,
}

impl From<stardust::NativeToken> for NativeToken {
    fn from(value: stardust::NativeToken) -> Self {
        let mut amount = Vec::with_capacity(size_of::<U256>());
        value.amount().to_little_endian(&mut amount);
        Self { token_id: value.token_id().to_vec().into_boxed_slice(), amount: amount.into_boxed_slice()}
    }
}

impl TryFrom<NativeToken> for stardust::NativeToken {
    type Error = Error;

    fn try_from(value: NativeToken) -> Result<Self, Self::Error> {
       // let token_id_bytes = value.token_id.as_ref().try_into()?;
       // Self::new(stardust::TokenId::new(token_id_bytes), U256::from_little_endian(value.amount.as_ref())?)
        todo!();
    }
}
