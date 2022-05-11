// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::input as stardust;
use serde::{Deserialize, Serialize};

use crate::types;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Input {
    #[serde(rename = "utxo")]
    Utxo(types::OutputId),
    #[serde(rename = "treasury")]
    Treasury(types::MilestoneId),
}

impl From<&stardust::Input> for Input {
    fn from(value: &stardust::Input) -> Self {
        match value {
            stardust::Input::Utxo(i) => Self::Utxo(i.output_id().into()),
            stardust::Input::Treasury(i) => Self::Treasury((*i.milestone_id()).into()),
        }
    }
}

impl TryFrom<Input> for stardust::Input {
    type Error = crate::types::error::Error;

    fn try_from(value: Input) -> Result<Self, Self::Error> {
        Ok(match value {
            Input::Utxo(i) => stardust::Input::Utxo(stardust::UtxoInput::new(i.transaction_id.try_into()?, i.index)?),
            Input::Treasury(i) => stardust::Input::Treasury(stardust::TreasuryInput::new(i.try_into()?)),
        })
    }
}
