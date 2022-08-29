// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{output as bee, rand::output::unlock_condition::rand_address_unlock_condition};

use super::{
    feature::{get_test_metadata_block, get_test_sender_block, get_test_tag_block},
    native_token::get_test_native_token,
    unlock_condition::*,
};
use crate::types::stardust::block::output::BasicOutput;

pub fn get_test_basic_output() -> BasicOutput {
    BasicOutput::from(
        &bee::BasicOutput::build_with_amount(100)
            .unwrap()
            .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
            .with_unlock_conditions([
                rand_address_unlock_condition().into(),
                rand_storage_deposit_return_unlock_condition().into(),
                rand_timelock_unlock_condition().into(),
                rand_expiration_unlock_condition().into(),
            ])
            .with_features(vec![
                get_test_sender_block(bee_block_stardust::rand::address::rand_address().into())
                    .try_into()
                    .unwrap(),
                get_test_metadata_block().try_into().unwrap(),
                get_test_tag_block().try_into().unwrap(),
            ])
            .finish()
            .unwrap(),
    )
}
