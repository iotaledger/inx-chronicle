// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    constant::TOKEN_SUPPLY,
    output::{NftId, NftOutput},
    rand::{
        bytes::rand_bytes_array, number::rand_number_range, output::unlock_condition::rand_address_unlock_condition,
    },
};

use super::{
    feature::rand_allowed_features,
    native_token::rand_native_tokens,
    unlock_condition::{
        rand_expiration_unlock_condition, rand_storage_deposit_return_unlock_condition, rand_timelock_unlock_condition,
    },
};

/// Generates a random [`NftId`].
pub fn rand_nft_id() -> NftId {
    rand_bytes_array().into()
}

/// Generates a random [`NftOutput`] with the given maximum amount and native token count.
pub fn rand_nft_output(max_amount: impl Into<Option<u64>>, max_native_tokens: impl Into<Option<u8>>) -> NftOutput {
    NftOutput::build_with_amount(
        rand_number_range(0..=max_amount.into().unwrap_or(TOKEN_SUPPLY)),
        rand_nft_id(),
    )
    .unwrap()
    .with_native_tokens(rand_native_tokens(max_native_tokens))
    .with_unlock_conditions([
        rand_address_unlock_condition().into(),
        rand_storage_deposit_return_unlock_condition().into(),
        rand_timelock_unlock_condition().into(),
        rand_expiration_unlock_condition().into(),
    ])
    .with_features(rand_allowed_features(NftOutput::ALLOWED_FEATURES))
    .with_immutable_features(rand_allowed_features(NftOutput::ALLOWED_IMMUTABLE_FEATURES))
    .finish()
    .unwrap()
}
