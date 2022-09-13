// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::signature as bee;

use crate::types::stardust::block::Signature;

pub fn get_test_signature() -> Signature {
    Signature::from(&bee::Signature::Ed25519(bee::Ed25519Signature::new(
        bee_block_stardust::rand::bytes::rand_bytes_array(),
        bee_block_stardust::rand::bytes::rand_bytes_array(),
    )))
}
