// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::input as stardust;
use serde::{Deserialize, Serialize};

use crate::types::stardust::message::{MilestoneId, OutputId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Input {
    #[serde(rename = "utxo")]
    Utxo(OutputId),
    #[serde(rename = "treasury")]
    Treasury { milestone_id: MilestoneId },
}

impl From<&stardust::Input> for Input {
    fn from(value: &stardust::Input) -> Self {
        match value {
            stardust::Input::Utxo(i) => Self::Utxo((*i.output_id()).into()),
            stardust::Input::Treasury(i) => Self::Treasury {
                milestone_id: (*i.milestone_id()).into(),
            },
        }
    }
}

impl TryFrom<Input> for stardust::Input {
    type Error = crate::types::error::Error;

    fn try_from(value: Input) -> Result<Self, Self::Error> {
        Ok(match value {
            Input::Utxo(i) => stardust::Input::Utxo(stardust::UtxoInput::new(i.transaction_id.try_into()?, i.index)?),
            Input::Treasury { milestone_id } => {
                stardust::Input::Treasury(stardust::TreasuryInput::new(milestone_id.try_into()?))
            }
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::message::{output::test::get_test_output_id, payload::test::get_test_milestone_id};

    #[test]
    fn test_input_bson() {
        let input = get_test_utxo_input();
        let bson = to_bson(&input).unwrap();
        from_bson::<Input>(bson).unwrap();

        let input = get_test_treasury_input();
        let bson = to_bson(&input).unwrap();
        from_bson::<Input>(bson).unwrap();
    }

    pub(crate) fn get_test_utxo_input() -> Input {
        Input::Utxo(get_test_output_id())
    }

    pub(crate) fn get_test_treasury_input() -> Input {
        Input::Treasury {
            milestone_id: get_test_milestone_id(),
        }
    }
}
