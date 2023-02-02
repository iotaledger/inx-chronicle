// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::*;
use crate::types::stardust::block::{Address, output::TokenAmount};

/// Measures activity of the base token, such as Shimmer or IOTA.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct BaseTokenActivityMeasurement {
    /// Represents the amount of tokens transfered. Tokens that are send back to an address are not counted.
    pub(crate) booked_value: TokenAmount,
    /// Represents the total amount of tokens transfered, independent of wether tokens were sent back to same address.
    pub(crate) transferred_value: TokenAmount,
}

impl Analytics for BaseTokenActivityMeasurement {
    type Measurement = PerMilestone<Self>;

    fn begin_milestone(&mut self, _ctx: &dyn AnalyticsContext) {
        *self = Default::default();
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        // The idea behind the following code is that we keep track of the deltas that are applied to each account that
        // is represented by an address.
        let mut balance_deltas: HashMap<&Address, TokenAmount> = HashMap::new();

        // We first gather all tokens that have been moved to an individual address.
        for output in created {
            if let Some(address) = output.owning_address() {
                *balance_deltas.entry(address).or_default() += output.amount();
            }
        }

        self.booked_value = balance_deltas.values().copied().sum();

        // Afterwards, we subtract the tokens from that address to get the actual deltas of each account.
        for input in consumed {
            if let Some(address) = input.owning_address() {
                *balance_deltas.entry(address).or_default() -= input.amount();
            }
        }

        // The number of transferred tokens is then the sum of all deltas.
        self.transferred_value = balance_deltas.values().copied().sum();
    }

    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        Some(PerMilestone {
            at: *ctx.at(),
            inner: *self,
        })
    }
}
