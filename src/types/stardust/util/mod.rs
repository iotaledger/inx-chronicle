// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust as bee;

use self::payload::{get_test_milestone_payload, get_test_tagged_data_payload, get_test_transaction_payload};
use super::block::Block;

pub mod input;
pub mod output;
pub mod payload;
pub mod signature;
pub mod unlock;

pub fn get_test_transaction_block() -> Block {
    Block::from(
        bee::BlockBuilder::<u64>::new(bee_block_stardust::rand::parents::rand_parents())
            .with_nonce_provider(u64::MAX, 0)
            .with_payload(get_test_transaction_payload().try_into().unwrap())
            .finish()
            .unwrap(),
    )
}

pub fn get_test_milestone_block() -> Block {
    Block::from(
        bee::BlockBuilder::<u64>::new(bee_block_stardust::rand::parents::rand_parents())
            .with_nonce_provider(u64::MAX, 0)
            .with_payload(get_test_milestone_payload().try_into().unwrap())
            .finish()
            .unwrap(),
    )
}

pub fn get_test_tagged_data_block() -> Block {
    Block::from(
        bee::BlockBuilder::<u64>::new(bee_block_stardust::rand::parents::rand_parents())
            .with_nonce_provider(u64::MAX, 0)
            .with_payload(get_test_tagged_data_payload().try_into().unwrap())
            .finish()
            .unwrap(),
    )
}
