// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::types::stardust::block::{output::Feature, Address};

pub fn get_test_sender_block(address: Address) -> Feature {
    Feature::Sender { address }
}

pub fn get_test_issuer_block(address: Address) -> Feature {
    Feature::Issuer { address }
}

pub fn get_test_metadata_block() -> Feature {
    Feature::Metadata {
        data: "Foo".as_bytes().to_vec().into_boxed_slice(),
    }
}

pub fn get_test_tag_block() -> Feature {
    Feature::Tag {
        data: "Bar".as_bytes().to_vec().into_boxed_slice(),
    }
}
