// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::payload as stardust;
use serde::{Deserialize, Serialize};

use super::MilestoneId;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreasuryTransactionPayload {
    input_milestone_id: MilestoneId,
    #[serde(with = "crate::dto::stringify")]
    output_amount: u64,
}

impl From<&stardust::TreasuryTransactionPayload> for TreasuryTransactionPayload {
    fn from(value: &stardust::TreasuryTransactionPayload) -> Self {
        Self {
            input_milestone_id: (*value.input().milestone_id()).into(),
            output_amount: value.output().amount(),
        }
    }
}

impl TryFrom<TreasuryTransactionPayload> for stardust::TreasuryTransactionPayload {
    type Error = crate::dto::error::Error;

    fn try_from(value: TreasuryTransactionPayload) -> Result<Self, Self::Error> {
        Ok(Self::new(
            bee_message_stardust::input::TreasuryInput::new(value.input_milestone_id.try_into()?),
            bee_message_stardust::output::TreasuryOutput::new(value.output_amount)?,
        )?)
    }
}
