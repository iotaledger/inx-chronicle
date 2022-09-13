// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Testing crate to add random generation functions for various types.

pub mod output;
pub mod payload;
pub mod signature;
pub mod unlock;

use bee_block_stardust::{rand::parents::rand_parents, Block, BlockBuilder};
use payload::{milestone::rand_milestone_payload, transaction::rand_transaction_payload};

/// Generates a random [`Block`] with a [`TransactionPayload`](bee_block_stardust::payload::TransactionPayload).
pub fn rand_transaction_block() -> Block {
    BlockBuilder::<u64>::new(rand_parents())
        .with_nonce_provider(u64::MAX, 0)
        .with_payload(rand_transaction_payload().into())
        .finish()
        .unwrap()
}

/// Generates a random [`Block`] with a [`MilestonePayload`](bee_block_stardust::payload::MilestonePayload).
pub fn rand_milestone_block(index: u32) -> Block {
    BlockBuilder::<u64>::new(rand_parents())
        .with_nonce_provider(u64::MAX, 0)
        .with_payload(rand_milestone_payload(index).into())
        .finish()
        .unwrap()
}
