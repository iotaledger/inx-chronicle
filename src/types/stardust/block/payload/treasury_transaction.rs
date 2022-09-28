// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::payload as bee;
use serde::{Deserialize, Serialize};

use super::milestone::MilestoneId;
use crate::types::context::TryFromWithContext;

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

impl TryFromWithContext<TreasuryTransactionPayload> for bee::TreasuryTransactionPayload {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(
        ctx: &bee_block_stardust::protocol::ProtocolParameters,
        value: TreasuryTransactionPayload,
    ) -> Result<Self, Self::Error> {
        Self::new(
            bee_block_stardust::input::TreasuryInput::new(value.input_milestone_id.into()),
            bee_block_stardust::output::TreasuryOutput::new(value.output_amount, ctx.token_supply())?,
        )
    }
}

impl From<TreasuryTransactionPayload> for bee::dto::TreasuryTransactionPayloadDto {
    fn from(value: TreasuryTransactionPayload) -> Self {
        Self {
            kind: bee::TreasuryTransactionPayload::KIND,
            input: bee_block_stardust::input::dto::InputDto::Treasury(
                bee_block_stardust::input::dto::TreasuryInputDto {
                    kind: bee_block_stardust::input::TreasuryInput::KIND,
                    milestone_id: value.input_milestone_id.to_hex(),
                },
            ),
            output: bee_block_stardust::output::dto::OutputDto::Treasury(
                bee_block_stardust::output::dto::TreasuryOutputDto {
                    kind: bee_block_stardust::output::TreasuryOutput::KIND,
                    amount: value.output_amount.to_string(),
                },
            ),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::payload::rand_treasury_transaction_payload;

    use super::*;

    impl TreasuryTransactionPayload {
        /// Generates a random [`TreasuryTransactionPayload`].
        pub fn rand(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            rand_treasury_transaction_payload(ctx.token_supply()).into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_treasury_transaction_payload_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let payload = TreasuryTransactionPayload::rand(&ctx);
        bee::TreasuryTransactionPayload::try_from_with_context(&ctx, payload).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<TreasuryTransactionPayload>(bson).unwrap());
    }
}
