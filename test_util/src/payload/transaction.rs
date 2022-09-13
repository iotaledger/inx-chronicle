// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    payload::{
        transaction::{RegularTransactionEssenceBuilder, TransactionEssence},
        TransactionPayload,
    },
    rand,
    rand::output::{rand_alias_output, rand_basic_output, rand_foundry_output, rand_nft_output},
    unlock::Unlocks,
};

use crate::unlock::{rand_alias_unlock, rand_nft_unlock, rand_reference_unlock, rand_signature_unlock};

pub fn rand_transaction_essence() -> TransactionEssence {
    TransactionEssence::Regular(
        RegularTransactionEssenceBuilder::new(0, [0; 32].into())
            .with_inputs(vec![
                rand::input::rand_utxo_input().into(),
                rand::input::rand_utxo_input().into(),
                rand::input::rand_utxo_input().into(),
                rand::input::rand_utxo_input().into(),
            ])
            .with_outputs(vec![
                rand_basic_output().into(),
                rand_alias_output().into(),
                rand_foundry_output().into(),
                rand_nft_output().into(),
            ])
            .finish()
            .unwrap(),
    )
}

pub fn rand_transaction_payload() -> TransactionPayload {
    TransactionPayload::new(
        rand_transaction_essence(),
        Unlocks::new(vec![
            rand_signature_unlock().into(),
            rand_reference_unlock().into(),
            rand_alias_unlock().into(),
            rand_nft_unlock().into(),
        ])
        .unwrap(),
    )
    .unwrap()
}
