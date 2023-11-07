// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::borrow::Borrow;

use iota_sdk::types::block::output::{NativeToken, TokenId};
use primitive_types::U256;
use serde::Deserialize;

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeTokenDto {
    /// The corresponding token id.
    pub token_id: TokenId,
    /// The amount of native tokens.
    pub amount: U256,
}

impl<T: Borrow<NativeToken>> From<T> for NativeTokenDto {
    fn from(value: T) -> Self {
        Self {
            token_id: *value.borrow().token_id(),
            amount: value.borrow().amount(),
        }
    }
}

impl TryFrom<NativeTokenDto> for NativeToken {
    type Error = iota_sdk::types::block::Error;

    fn try_from(value: NativeTokenDto) -> Result<Self, Self::Error> {
        Self::new(value.token_id.into(), value.amount)
    }
}
