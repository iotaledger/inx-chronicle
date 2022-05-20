// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as stardust;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreasuryOutput {
    #[serde(with = "crate::types::stringify")]
    amount: u64,
}

impl From<&stardust::TreasuryOutput> for TreasuryOutput {
    fn from(value: &stardust::TreasuryOutput) -> Self {
        Self { amount: value.amount() }
    }
}

impl TryFrom<TreasuryOutput> for stardust::TreasuryOutput {
    type Error = crate::types::error::Error;

    fn try_from(value: TreasuryOutput) -> Result<Self, Self::Error> {
        Ok(Self::new(value.amount)?)
    }
}
