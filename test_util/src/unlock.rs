// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    rand::number::rand_number_range,
    unlock::{AliasUnlock, NftUnlock, ReferenceUnlock, SignatureUnlock, UNLOCK_INDEX_RANGE},
};

use super::signature::rand_signature;

/// Generates a random [`SignatureUnlock`].
pub fn rand_signature_unlock() -> SignatureUnlock {
    SignatureUnlock::new(rand_signature())
}

/// Generates a random [`ReferenceUnlock`].
pub fn rand_reference_unlock() -> ReferenceUnlock {
    ReferenceUnlock::new(rand_number_range(UNLOCK_INDEX_RANGE)).unwrap()
}

/// Generates a random [`AliasUnlock`].
pub fn rand_alias_unlock() -> AliasUnlock {
    AliasUnlock::new(rand_number_range(UNLOCK_INDEX_RANGE)).unwrap()
}

/// Generates a random [`NftUnlock`].
pub fn rand_nft_unlock() -> NftUnlock {
    NftUnlock::new(rand_number_range(UNLOCK_INDEX_RANGE)).unwrap()
}
