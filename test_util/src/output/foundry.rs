// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    constant::TOKEN_SUPPLY,
    output::FoundryOutput,
    rand::{
        number::{rand_number, rand_number_range},
        output::rand_token_scheme,
    },
};

use super::{
    feature::rand_allowed_features, native_token::rand_native_tokens,
    unlock_condition::rand_immutable_alias_address_unlock_condition,
};

/// Generates a random [`FoundryOutput`] with the given maximum amount and native token count.
pub fn rand_foundry_output(
    max_amount: impl Into<Option<u64>>,
    max_native_tokens: impl Into<Option<u8>>,
) -> FoundryOutput {
    FoundryOutput::build_with_amount(
        rand_number_range(0..=max_amount.into().unwrap_or(TOKEN_SUPPLY)),
        rand_number(),
        rand_token_scheme(),
    )
    .unwrap()
    .with_native_tokens(rand_native_tokens(max_native_tokens))
    .with_unlock_conditions([rand_immutable_alias_address_unlock_condition().into()])
    .with_features(rand_allowed_features(FoundryOutput::ALLOWED_FEATURES))
    .with_immutable_features(rand_allowed_features(FoundryOutput::ALLOWED_IMMUTABLE_FEATURES))
    .finish()
    .unwrap()
}
