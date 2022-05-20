// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::input as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::{MilestoneId, OutputId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Input {
    #[serde(rename = "utxo")]
    Utxo(OutputId),
    #[serde(rename = "treasury")]
    Treasury { milestone_id: MilestoneId },
}

impl From<&bee::Input> for Input {
    fn from(value: &bee::Input) -> Self {
        match value {
            bee::Input::Utxo(i) => Self::Utxo((*i.output_id()).into()),
            bee::Input::Treasury(i) => Self::Treasury {
                milestone_id: (*i.milestone_id()).into(),
            },
        }
    }
}

impl TryFrom<Input> for bee::Input {
    type Error = crate::types::error::Error;

    fn try_from(value: Input) -> Result<Self, Self::Error> {
        Ok(match value {
            Input::Utxo(i) => bee::Input::Utxo(bee::UtxoInput::new(i.transaction_id.try_into()?, i.index)?),
            Input::Treasury { milestone_id } => bee::Input::Treasury(bee::TreasuryInput::new(milestone_id.try_into()?)),
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_input_bson() {
        let input = get_test_utxo_input();
        let bson = to_bson(&input).unwrap();
        assert_eq!(input, from_bson::<Input>(bson).unwrap());

        let input = get_test_treasury_input();
        let bson = to_bson(&input).unwrap();
        assert_eq!(input, from_bson::<Input>(bson).unwrap());
    }

    pub(crate) fn get_test_utxo_input() -> Input {
        Input::Utxo(bee_test::rand::output::rand_output_id().into())
    }

    pub(crate) fn get_test_treasury_input() -> Input {
        Input::Treasury {
            milestone_id: bee_test::rand::milestone::rand_milestone_id().into(),
        }
    }
}
