// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use bee_block_stardust::{
    output::{NativeToken, NativeTokens, TokenId},
    rand::{
        bytes::{rand_bytes, rand_bytes_array},
        number::rand_number_range,
    },
};
use primitive_types::U256;

/// Generates a random [`TokenId`].
pub fn rand_token_id() -> TokenId {
    rand_bytes_array().into()
}

/// Generates a random [`NativeToken`].
pub fn rand_native_token() -> NativeToken {
    NativeToken::new(rand_token_id(), rand_token_amount()).unwrap()
}

/// Generates random [`NativeTokens`] with the given maximum.
pub fn rand_native_tokens(max_tokens: impl Into<Option<u8>>) -> NativeTokens {
    let mut tokens = HashMap::new();
    let count = rand_number_range(1..=max_tokens.into().unwrap_or(NativeTokens::COUNT_MAX).max(1)) as usize;
    while tokens.len() < count {
        tokens.insert(rand_token_id(), rand_token_amount());
    }
    NativeTokens::new(
        tokens
            .into_iter()
            .map(|(id, amt)| NativeToken::new(id, amt).unwrap())
            .collect(),
    )
    .unwrap()
}

/// Generates a random token amount.
pub fn rand_token_amount() -> U256 {
    U256::from_little_endian(&rand_bytes(32)).max(1.into())
}
