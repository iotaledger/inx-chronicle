// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    constant::TOKEN_SUPPLY,
    payload::{
        transaction::{RegularTransactionEssenceBuilder, TransactionEssence},
        TransactionPayload,
    },
    rand::{input::rand_utxo_input, output::rand_inputs_commitment},
    unlock::{AliasUnlock, NftUnlock, ReferenceUnlock, Unlocks},
};

use crate::{
    output::{rand_alias_output, rand_basic_output, rand_foundry_output, rand_nft_output},
    unlock::rand_signature_unlock,
};

/// Generates a random [`TransactionEssence`].
pub fn rand_transaction_essence() -> TransactionEssence {
    RegularTransactionEssenceBuilder::new(0, rand_inputs_commitment())
        .with_inputs(vec![
            rand_utxo_input().into(),
            rand_utxo_input().into(),
            rand_utxo_input().into(),
            rand_utxo_input().into(),
        ])
        .with_outputs(vec![
            rand_basic_output(TOKEN_SUPPLY / 4, 3).into(),
            rand_alias_output(TOKEN_SUPPLY / 4, 3).into(),
            rand_foundry_output(TOKEN_SUPPLY / 4, 3).into(),
            rand_nft_output(TOKEN_SUPPLY / 4, 3).into(),
        ])
        .finish()
        .unwrap()
        .into()
}

/// Generates a random [`TransactionPayload`].
pub fn rand_transaction_payload() -> TransactionPayload {
    TransactionPayload::new(
        rand_transaction_essence(),
        Unlocks::new(vec![
            rand_signature_unlock().into(),
            ReferenceUnlock::new(0).unwrap().into(),
            AliasUnlock::new(0).unwrap().into(),
            NftUnlock::new(0).unwrap().into(),
        ])
        .unwrap(),
    )
    .unwrap()
}
