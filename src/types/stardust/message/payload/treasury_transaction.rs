// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::payload as bee;
use serde::{Deserialize, Serialize};

use super::MilestoneId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryTransactionPayload {
    input_milestone_id: MilestoneId,
    #[serde(with = "crate::types::stringify")]
    output_amount: u64,
}

impl From<&bee::TreasuryTransactionPayload> for TreasuryTransactionPayload {
    fn from(value: &bee::TreasuryTransactionPayload) -> Self {
        Self {
            input_milestone_id: (*value.input().milestone_id()).into(),
            output_amount: value.output().amount(),
        }
    }
}

impl TryFrom<TreasuryTransactionPayload> for bee::TreasuryTransactionPayload {
    type Error = crate::types::error::Error;

    fn try_from(value: TreasuryTransactionPayload) -> Result<Self, Self::Error> {
        Ok(Self::new(
            bee_message_stardust::input::TreasuryInput::new(value.input_milestone_id.try_into()?),
            bee_message_stardust::output::TreasuryOutput::new(value.output_amount)?,
        )?)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_treasury_transaction_payload_bson() {
        let payload = get_test_treasury_transaction_payload();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<TreasuryTransactionPayload>(bson).unwrap());
    }

    pub(crate) fn get_test_treasury_transaction_payload() -> TreasuryTransactionPayload {
        TreasuryTransactionPayload::from(
            &bee::TreasuryTransactionPayload::new(
                bee_test::rand::input::rand_treasury_input(),
                bee_test::rand::output::rand_treasury_output(),
            )
            .unwrap(),
        )
    }
}
