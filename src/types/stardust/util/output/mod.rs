// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::types::stardust::block::Output;

pub mod alias;
pub mod basic;
pub mod feature;
pub mod foundry;
pub mod native_token;
pub mod nft;
pub mod unlock_condition;

pub fn get_test_alias_output() -> Output {
    Output::Alias(alias::get_test_alias_output())
}

pub fn get_test_basic_output() -> Output {
    Output::Basic(basic::get_test_basic_output())
}

pub fn get_test_foundry_output() -> Output {
    Output::Foundry(foundry::get_test_foundry_output())
}

pub fn get_test_nft_output() -> Output {
    Output::Nft(nft::get_test_nft_output())
}
