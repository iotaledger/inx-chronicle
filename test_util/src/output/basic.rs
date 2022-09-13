// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    constant::TOKEN_SUPPLY,
    output::BasicOutput,
    rand::{number::rand_number_range, output::unlock_condition::rand_address_unlock_condition},
};

use super::{
    feature::rand_allowed_features,
    native_token::rand_native_tokens,
    unlock_condition::{
        rand_expiration_unlock_condition, rand_storage_deposit_return_unlock_condition, rand_timelock_unlock_condition,
    },
};

/// Generates a random [`BasicOutput`] with the given maximum amount and native token count.
pub fn rand_basic_output(max_amount: impl Into<Option<u64>>, max_native_tokens: impl Into<Option<u8>>) -> BasicOutput {
    BasicOutput::build_with_amount(rand_number_range(0..max_amount.into().unwrap_or(TOKEN_SUPPLY)))
        .unwrap()
        .with_native_tokens(rand_native_tokens(max_native_tokens))
        .with_unlock_conditions([
            rand_address_unlock_condition().into(),
            rand_storage_deposit_return_unlock_condition().into(),
            rand_timelock_unlock_condition().into(),
            rand_expiration_unlock_condition().into(),
        ])
        .with_features(rand_allowed_features(BasicOutput::ALLOWED_FEATURES))
        .finish()
        .unwrap()
}
