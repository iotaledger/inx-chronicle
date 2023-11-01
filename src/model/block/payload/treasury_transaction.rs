// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Contains the [`TreasuryTransactionPayload`].

use std::borrow::Borrow;

use iota_sdk::types::block::payload as iota;
use serde::{Deserialize, Serialize};

use super::milestone::MilestoneId;
use crate::model::{stringify, TryFromWithContext};

/// Represents a treasury transaction payload.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryTransactionPayload {
    /// The milestone id of the input.
    pub input_milestone_id: MilestoneId,
    /// The amount of tokens in output.
    #[serde(with = "stringify")]
    pub output_amount: u64,
}

impl TreasuryTransactionPayload {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "treasury_transaction";
}

impl<T: Borrow<iota::TreasuryTransactionPayload>> From<T> for TreasuryTransactionPayload {
    fn from(value: T) -> Self {
        Self {
            input_milestone_id: (*value.borrow().input().milestone_id()).into(),
            output_amount: value.borrow().output().amount(),
        }
    }
}

impl TryFromWithContext<TreasuryTransactionPayload> for iota::TreasuryTransactionPayload {
    type Error = iota_sdk::types::block::Error;

    fn try_from_with_context(
        ctx: &iota_sdk::types::block::protocol::ProtocolParameters,
        value: TreasuryTransactionPayload,
    ) -> Result<Self, Self::Error> {
        Self::new(
            iota_sdk::types::block::input::TreasuryInput::new(value.input_milestone_id.into()),
            iota_sdk::types::block::output::TreasuryOutput::new(value.output_amount, ctx.token_supply())?,
        )
    }
}

impl From<TreasuryTransactionPayload> for iota::dto::TreasuryTransactionPayloadDto {
    fn from(value: TreasuryTransactionPayload) -> Self {
        Self {
            kind: iota::TreasuryTransactionPayload::KIND,
            input: iota_sdk::types::block::input::dto::InputDto::Treasury(
                iota_sdk::types::block::input::dto::TreasuryInputDto {
                    kind: iota_sdk::types::block::input::TreasuryInput::KIND,
                    milestone_id: value.input_milestone_id.to_hex(),
                },
            ),
            output: iota_sdk::types::block::output::dto::OutputDto::Treasury(
                iota_sdk::types::block::output::dto::TreasuryOutputDto {
                    kind: iota_sdk::types::block::output::TreasuryOutput::KIND,
                    amount: value.output_amount.to_string(),
                },
            ),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_sdk::types::block::rand::payload::rand_treasury_transaction_payload;

    use super::*;

    impl TreasuryTransactionPayload {
        /// Generates a random [`TreasuryTransactionPayload`].
        pub fn rand(ctx: &iota_sdk::types::block::protocol::ProtocolParameters) -> Self {
            rand_treasury_transaction_payload(ctx.token_supply()).into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_treasury_transaction_payload_bson() {
        let ctx = iota_sdk::types::block::protocol::protocol_parameters();
        let payload = TreasuryTransactionPayload::rand(&ctx);
        iota::TreasuryTransactionPayload::try_from_with_context(&ctx, payload).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<TreasuryTransactionPayload>(bson).unwrap());
    }
}
