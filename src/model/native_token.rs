// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the native token.

use core::borrow::Borrow;

use iota_sdk::types::block::output::{NativeToken, NativeTokenError, TokenId};
use primitive_types::U256;
use serde::{Deserialize, Serialize};

/// A native token.
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
    type Error = NativeTokenError;

    fn try_from(value: NativeTokenDto) -> Result<Self, Self::Error> {
        Self::new(value.token_id, value.amount)
    }
}
