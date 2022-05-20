// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as bee;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryOutput {
    #[serde(with = "crate::types::stringify")]
    amount: u64,
}

impl From<&bee::TreasuryOutput> for TreasuryOutput {
    fn from(value: &bee::TreasuryOutput) -> Self {
        Self { amount: value.amount() }
    }
}

impl TryFrom<TreasuryOutput> for bee::TreasuryOutput {
    type Error = crate::types::error::Error;

    fn try_from(value: TreasuryOutput) -> Result<Self, Self::Error> {
        Ok(Self::new(value.amount)?)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_treasury_output_bson() {
        let output = TreasuryOutput::from(&bee_test::rand::output::rand_treasury_output());
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<TreasuryOutput>(bson).unwrap());
    }
}
