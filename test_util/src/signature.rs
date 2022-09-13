// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    rand::bytes::rand_bytes_array,
    signature::{Ed25519Signature, Signature},
};

/// Generates a random [`Signature`] with an [`Ed25519Signature`].
pub fn rand_signature() -> Signature {
    Signature::Ed25519(Ed25519Signature::new(rand_bytes_array(), rand_bytes_array()))
}
