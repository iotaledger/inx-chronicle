// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    constant::TOKEN_SUPPLY,
    output::AliasOutput,
    rand::{
        number::rand_number_range,
        output::{
            rand_alias_id,
            unlock_condition::{
                rand_governor_address_unlock_condition_different_from,
                rand_state_controller_address_unlock_condition_different_from,
            },
        },
    },
};

use super::{feature::rand_allowed_features, native_token::rand_native_tokens};

/// Generates a random [`AliasOutput`] with the given maximum amount and native token count.
pub fn rand_alias_output(max_amount: impl Into<Option<u64>>, max_native_tokens: impl Into<Option<u8>>) -> AliasOutput {
    let alias_id = rand_alias_id();
    AliasOutput::build_with_amount(
        rand_number_range(0..=max_amount.into().unwrap_or(TOKEN_SUPPLY)),
        alias_id,
    )
    .unwrap()
    .with_native_tokens(rand_native_tokens(max_native_tokens))
    .with_state_index(0)
    .with_state_metadata("Foo".as_bytes().to_vec())
    .with_foundry_counter(0)
    .with_unlock_conditions([
        rand_state_controller_address_unlock_condition_different_from(&alias_id).into(),
        rand_governor_address_unlock_condition_different_from(&alias_id).into(),
    ])
    .with_features(rand_allowed_features(AliasOutput::ALLOWED_FEATURES))
    .with_immutable_features(rand_allowed_features(AliasOutput::ALLOWED_IMMUTABLE_FEATURES))
    .finish()
    .unwrap()
}
