// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    output::{NativeToken, TokenId},
    rand,
};

pub fn rand_token_id() -> TokenId {
    rand::bytes::rand_bytes_array().into()
}

pub fn rand_native_token() -> NativeToken {
    NativeToken::new(rand::bytes::rand_bytes_array().into(), 100.into()).unwrap()
}
