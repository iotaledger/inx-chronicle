// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as bee;

use crate::types::stardust::block::output::NativeToken;

pub fn rand_token_id() -> bee::TokenId {
    bee_block_stardust::rand::bytes::rand_bytes_array().into()
}

pub fn get_test_native_token() -> NativeToken {
    NativeToken::from(
        &bee::NativeToken::new(bee_block_stardust::rand::bytes::rand_bytes_array().into(), 100.into()).unwrap(),
    )
}
