// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::signature::get_test_signature;
use crate::types::stardust::block::Unlock;

pub fn get_test_signature_unlock() -> Unlock {
    Unlock::Signature {
        signature: get_test_signature(),
    }
}

pub fn get_test_reference_unlock() -> Unlock {
    Unlock::Reference { index: 0 }
}

pub fn get_test_alias_unlock() -> Unlock {
    Unlock::Alias { index: 0 }
}

pub fn get_test_nft_unlock() -> Unlock {
    Unlock::Nft { index: 0 }
}
