// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as bee;

use super::{
    feature::get_test_metadata_block, native_token::get_test_native_token,
    unlock_condition::rand_immutable_alias_address_unlock_condition,
};
use crate::types::stardust::block::output::FoundryOutput;

pub fn get_test_foundry_output() -> FoundryOutput {
    FoundryOutput::from(
        &bee::FoundryOutput::build_with_amount(
            100,
            bee_block_stardust::rand::number::rand_number(),
            bee::TokenScheme::Simple(bee::SimpleTokenScheme::new(250.into(), 200.into(), 300.into()).unwrap()),
        )
        .unwrap()
        .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
        .with_unlock_conditions([rand_immutable_alias_address_unlock_condition().into()])
        .with_features(vec![get_test_metadata_block().try_into().unwrap()])
        .with_immutable_features(vec![get_test_metadata_block().try_into().unwrap()])
        .finish()
        .unwrap(),
    )
}
