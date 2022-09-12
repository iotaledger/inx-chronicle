// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::payload as bee;

use crate::types::stardust::block::payload::TreasuryTransactionPayload;

pub fn get_test_treasury_transaction_payload() -> TreasuryTransactionPayload {
    TreasuryTransactionPayload::from(
        &bee::TreasuryTransactionPayload::new(
            bee_block_stardust::rand::input::rand_treasury_input(),
            bee_block_stardust::rand::output::rand_treasury_output(),
        )
        .unwrap(),
    )
}
