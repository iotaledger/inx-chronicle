// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::types::stardust::block::Input;

pub fn get_test_utxo_input() -> Input {
    Input::Utxo(bee_block_stardust::rand::output::rand_output_id().into())
}

pub fn get_test_treasury_input() -> Input {
    Input::Treasury {
        milestone_id: bee_block_stardust::rand::milestone::rand_milestone_id().into(),
    }
}
