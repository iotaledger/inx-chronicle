// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::signature::{Ed25519Signature, Signature};

pub fn rand_signature() -> Signature {
    Signature::Ed25519(Ed25519Signature::new(
        bee_block_stardust::rand::bytes::rand_bytes_array(),
        bee_block_stardust::rand::bytes::rand_bytes_array(),
    ))
}
