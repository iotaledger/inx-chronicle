// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::types::stardust::block::payload::TaggedDataPayload;

pub fn get_test_tagged_data_payload() -> TaggedDataPayload {
    Into::into(&bee_block_stardust::rand::payload::rand_tagged_data_payload())
}
