// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::unlock::{AliasUnlock, NftUnlock, ReferenceUnlock, SignatureUnlock};

use super::signature::rand_signature;

pub fn rand_signature_unlock() -> SignatureUnlock {
    SignatureUnlock::new(rand_signature())
}

pub fn rand_reference_unlock() -> ReferenceUnlock {
    ReferenceUnlock::new(0).unwrap()
}

pub fn rand_alias_unlock() -> AliasUnlock {
    AliasUnlock::new(0).unwrap()
}

pub fn rand_nft_unlock() -> NftUnlock {
    NftUnlock::new(0).unwrap()
}
