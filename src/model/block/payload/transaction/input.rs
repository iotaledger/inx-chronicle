// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Input`] type.

use iota_sdk::types::block::{input as iota, output::OutputId};
use serde::{Deserialize, Serialize};

/// The type for [`Inputs`](Input) in the UTXO model.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum InputDto {
    /// The id of the corresponding output.
    Utxo { output_id: OutputId },
}

impl From<&iota::Input> for InputDto {
    fn from(value: &iota::Input) -> Self {
        match value {
            iota::Input::Utxo(i) => Self::Utxo {
                output_id: *i.output_id(),
            },
        }
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_utxo_input_bson() {
//         let input = Input::rand_utxo();
//         let bson = to_bson(&input).unwrap();
//         assert_eq!(input, from_bson::<Input>(bson).unwrap());
//     }

//     #[test]
//     fn test_treasury_input_bson() {
//         let input = Input::rand_treasury();
//         let bson = to_bson(&input).unwrap();
//         assert_eq!(input, from_bson::<Input>(bson).unwrap());
//     }
// }
