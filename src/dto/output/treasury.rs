// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output as stardust;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TreasuryOutput {
    amount: u64,
}

impl From<stardust::TreasuryOutput> for TreasuryOutput {
    fn from(value: stardust::TreasuryOutput) -> Self {
        Self { amount: value.amount() }
    }
}

impl TryFrom<TreasuryOutput> for stardust::TreasuryOutput {
    type Error = bee_message_stardust::Error;

    fn try_from(value: TreasuryOutput) -> Result<Self, Self::Error> {
        Self::new(value.amount)
    }
}
