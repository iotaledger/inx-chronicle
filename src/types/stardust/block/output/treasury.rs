// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::output as bee;
use serde::{Deserialize, Serialize};

use super::OutputAmount;
use crate::types::context::TryFromWithContext;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryOutput {
    pub amount: OutputAmount,
}

impl<T: Borrow<bee::TreasuryOutput>> From<T> for TreasuryOutput {
    fn from(value: T) -> Self {
        Self {
            amount: value.borrow().amount().into(),
        }
    }
}

impl TryFromWithContext<TreasuryOutput> for bee::TreasuryOutput {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(
        ctx: &bee_block_stardust::protocol::ProtocolParameters,
        value: TreasuryOutput,
    ) -> Result<Self, Self::Error> {
        Self::new(value.amount.0, ctx.token_supply())
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::output::rand_treasury_output;

    use super::*;

    impl TreasuryOutput {
        /// Generates a random [`TreasuryOutput`].
        pub fn rand(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            rand_treasury_output(ctx.token_supply()).into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_treasury_output_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let output = TreasuryOutput::rand(&ctx);
        bee::TreasuryOutput::try_from_with_context(&ctx, output).unwrap();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<TreasuryOutput>(bson).unwrap());
    }
}
