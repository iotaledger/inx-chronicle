// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::input::Input;

pub fn rand_utxo_input() -> Input {
    Input::Utxo(bee_block_stardust::rand::output::rand_output_id().into())
}

pub fn rand_treasury_input() -> Input {
    Input::Treasury(bee_block_stardust::rand::milestone::rand_milestone_id().into())
}
