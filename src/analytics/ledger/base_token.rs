// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use iota_sdk::types::block::{address::Address, payload::SignedTransactionPayload};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::{
        block_metadata::TransactionMetadata,
        ledger::{LedgerOutput, LedgerSpent},
    },
};

/// Measures activity of the base token, such as Shimmer or IOTA.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct BaseTokenActivityMeasurement {
    /// Represents the amount of tokens transferred. Tokens that are send back to an address are not counted.
    pub(crate) booked_amount: u64,
    /// Represents the total amount of tokens transferred, independent of whether tokens were sent back to same
    /// address.
    pub(crate) transferred_amount: u64,
}

#[async_trait::async_trait]
impl Analytics for BaseTokenActivityMeasurement {
    type Measurement = Self;

    async fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        _metadata: &TransactionMetadata,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        // The idea behind the following code is that we keep track of the deltas that are applied to each account that
        // is represented by an address.
        let mut balance_deltas: HashMap<Address, i128> = HashMap::new();

        // We first gather all tokens that have been moved to an individual address.
        for output in created {
            *balance_deltas
                .entry(output.locked_address_at(ctx.slot_index(), ctx.protocol_parameters()))
                .or_default() += output.amount() as i128;
        }

        self.booked_amount += balance_deltas.values().sum::<i128>() as u64;

        // Afterwards, we subtract the tokens from that address to get the actual deltas of each account.
        for output in consumed {
            *balance_deltas
                .entry(output.locked_address_at(ctx.slot_index(), ctx.protocol_parameters()))
                .or_default() -= output.amount() as i128;
        }

        // The number of transferred tokens is then the sum of all deltas.
        self.transferred_amount += balance_deltas.values().copied().map(|d| d.max(0) as u64).sum::<u64>();

        Ok(())
    }

    async fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        Ok(std::mem::take(self))
    }
}
