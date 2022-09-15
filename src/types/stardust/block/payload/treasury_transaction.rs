// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::payload as bee;
use serde::{Deserialize, Serialize};

use super::milestone::MilestoneId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryTransactionPayload {
    pub input_milestone_id: MilestoneId,
    #[serde(with = "crate::types::util::stringify")]
    pub output_amount: u64,
}

impl<T: Borrow<bee::TreasuryTransactionPayload>> From<T> for TreasuryTransactionPayload {
    fn from(value: T) -> Self {
        Self {
            input_milestone_id: (*value.borrow().input().milestone_id()).into(),
            output_amount: value.borrow().output().amount(),
        }
    }
}

impl TryFrom<TreasuryTransactionPayload> for bee::TreasuryTransactionPayload {
    type Error = bee_block_stardust::Error;

    fn try_from(value: TreasuryTransactionPayload) -> Result<Self, Self::Error> {
        Self::new(
            bee_block_stardust::input::TreasuryInput::new(value.input_milestone_id.into()),
            bee_block_stardust::output::TreasuryOutput::new(value.output_amount)?,
        )
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::payload::rand_treasury_transaction_payload;

    use super::*;

    impl TreasuryTransactionPayload {
        /// Generates a random [`TreasuryTransactionPayload`].
        pub fn rand() -> Self {
            rand_treasury_transaction_payload().into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_treasury_transaction_payload_bson() {
        let payload = TreasuryTransactionPayload::rand();
        bee::TreasuryTransactionPayload::try_from(payload).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<TreasuryTransactionPayload>(bson).unwrap());
    }
}
