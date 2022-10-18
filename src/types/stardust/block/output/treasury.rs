// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`TreasuryOutput`].

use std::borrow::Borrow;

use iota_types::block::output as iota;
use serde::{Deserialize, Serialize};

use super::OutputAmount;
use crate::types::context::TryFromWithContext;

/// Represents a treasury in the UTXO model.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryOutput {
    /// The output amount.
    pub amount: OutputAmount,
}

impl<T: Borrow<iota::TreasuryOutput>> From<T> for TreasuryOutput {
    fn from(value: T) -> Self {
        Self {
            amount: value.borrow().amount().into(),
        }
    }
}

impl TryFromWithContext<TreasuryOutput> for iota::TreasuryOutput {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: TreasuryOutput,
    ) -> Result<Self, Self::Error> {
        Self::new(value.amount.0, ctx.token_supply())
    }
}

impl From<TreasuryOutput> for iota::dto::TreasuryOutputDto {
    fn from(value: TreasuryOutput) -> Self {
        Self {
            kind: iota::TreasuryOutput::KIND,
            amount: value.amount.0.to_string(),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::output::rand_treasury_output;

    use super::*;

    impl TreasuryOutput {
        /// Generates a random [`TreasuryOutput`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
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
        let ctx = iota_types::block::protocol::protocol_parameters();
        let output = TreasuryOutput::rand(&ctx);
        iota::TreasuryOutput::try_from_with_context(&ctx, output).unwrap();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<TreasuryOutput>(bson).unwrap());
    }
}
