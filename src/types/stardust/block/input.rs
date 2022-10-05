// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Input`] type.

use bee_block_stardust::input as bee;
use serde::{Deserialize, Serialize};

use super::{output::OutputId, payload::milestone::MilestoneId};

/// The type for [`Input`]s in the UTXO model.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Input {
    /// The id of the corresponding output.
    Utxo(OutputId),
    /// A treasury that corresponds to a milestone.
    Treasury {
        /// The [`MilestoneId`] corresponding to the treasury.
        milestone_id: MilestoneId,
    },
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
    type Error = bee_block_stardust::Error;

    fn try_from(value: Input) -> Result<Self, Self::Error> {
        Ok(match value {
            Input::Utxo(i) => bee::Input::Utxo(bee::UtxoInput::new(i.transaction_id.into(), i.index)?),
            Input::Treasury { milestone_id } => bee::Input::Treasury(bee::TreasuryInput::new(milestone_id.into())),
        })
    }
}

impl From<Input> for bee::dto::InputDto {
    fn from(value: Input) -> Self {
        match value {
            Input::Utxo(output_id) => Self::Utxo(bee::dto::UtxoInputDto {
                kind: bee::UtxoInput::KIND,
                transaction_id: output_id.transaction_id.to_hex(),
                transaction_output_index: output_id.index,
            }),
            Input::Treasury { milestone_id } => Self::Treasury(bee::dto::TreasuryInputDto {
                kind: bee::TreasuryInput::KIND,
                milestone_id: milestone_id.to_hex(),
            }),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {

    use bee_block_stardust::rand::{
        input::{rand_treasury_input, rand_utxo_input},
        number::rand_number_range,
    };

    use super::*;

    impl Input {
        /// Generates a random [`Input`].
        pub fn rand() -> Self {
            match rand_number_range(0..2) {
                0 => Self::rand_utxo(),
                1 => Self::rand_treasury(),
                _ => unreachable!(),
            }
        }

        /// Generates a random utxo [`Input`].
        pub fn rand_utxo() -> Self {
            Self::from(&bee::Input::from(rand_utxo_input()))
        }

        /// Generates a random treasury [`Input`].
        pub fn rand_treasury() -> Self {
            Self::from(&bee::Input::from(rand_treasury_input()))
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_utxo_input_bson() {
        let input = Input::rand_utxo();
        let bson = to_bson(&input).unwrap();
        assert_eq!(input, from_bson::<Input>(bson).unwrap());
    }

    #[test]
    fn test_treasury_input_bson() {
        let input = Input::rand_treasury();
        let bson = to_bson(&input).unwrap();
        assert_eq!(input, from_bson::<Input>(bson).unwrap());
    }
}
