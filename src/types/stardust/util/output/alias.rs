// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as bee;

use super::{feature::*, native_token::get_test_native_token, unlock_condition::*};
use crate::types::stardust::block::output::AliasOutput;

pub fn get_test_alias_output() -> AliasOutput {
    AliasOutput::from(
        &bee::AliasOutput::build_with_amount(100, bee_block_stardust::rand::output::rand_alias_id())
            .unwrap()
            .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
            .with_state_index(0)
            .with_state_metadata("Foo".as_bytes().to_vec())
            .with_foundry_counter(0)
            .with_unlock_conditions([
                rand_state_controller_address_unlock_condition().into(),
                rand_governor_address_unlock_condition().into(),
            ])
            .with_features(vec![
                get_test_sender_block(bee_block_stardust::rand::address::rand_address().into())
                    .try_into()
                    .unwrap(),
                get_test_metadata_block().try_into().unwrap(),
            ])
            .with_immutable_features(vec![
                get_test_issuer_block(bee_block_stardust::rand::address::rand_address().into())
                    .try_into()
                    .unwrap(),
                get_test_metadata_block().try_into().unwrap(),
            ])
            .finish()
            .unwrap(),
    )
}
