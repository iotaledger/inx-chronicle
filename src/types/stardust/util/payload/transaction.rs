// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::payload::transaction as bee;

use crate::types::stardust::{
    block::{
        output::OutputId,
        payload::{TransactionEssence, TransactionPayload},
        Input,
    },
    util::{output::*, unlock::*},
};

pub(crate) const OUTPUT_ID1: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c6492a00";
pub(crate) const OUTPUT_ID2: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c6492b00";
pub(crate) const OUTPUT_ID3: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c6492c00";
pub(crate) const OUTPUT_ID4: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c6492d00";

pub fn get_test_transaction_essence() -> TransactionEssence {
    TransactionEssence::from(&bee::TransactionEssence::Regular(
        bee::RegularTransactionEssenceBuilder::new(0, [0; 32].into())
            .with_inputs(vec![
                Input::Utxo(OutputId::from_str(OUTPUT_ID1).unwrap()).try_into().unwrap(),
                Input::Utxo(OutputId::from_str(OUTPUT_ID2).unwrap()).try_into().unwrap(),
                Input::Utxo(OutputId::from_str(OUTPUT_ID3).unwrap()).try_into().unwrap(),
                Input::Utxo(OutputId::from_str(OUTPUT_ID4).unwrap()).try_into().unwrap(),
            ])
            .with_outputs(vec![
                get_test_basic_output().try_into().unwrap(),
                get_test_alias_output().try_into().unwrap(),
                get_test_foundry_output().try_into().unwrap(),
                get_test_nft_output().try_into().unwrap(),
            ])
            .finish()
            .unwrap(),
    ))
}

pub fn get_test_transaction_payload() -> TransactionPayload {
    TransactionPayload::from(
        &bee::TransactionPayload::new(
            get_test_transaction_essence().try_into().unwrap(),
            bee_block_stardust::unlock::Unlocks::new(vec![
                get_test_signature_unlock().try_into().unwrap(),
                get_test_reference_unlock().try_into().unwrap(),
                get_test_alias_unlock().try_into().unwrap(),
                get_test_nft_unlock().try_into().unwrap(),
            ])
            .unwrap(),
        )
        .unwrap(),
    )
}
