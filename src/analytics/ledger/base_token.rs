// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use iota_sdk::types::block::address::{Bech32Address, ToBech32Ext};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::ledger::{LedgerOutput, LedgerSpent},
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

impl Analytics for BaseTokenActivityMeasurement {
    type Measurement = Self;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], ctx: &dyn AnalyticsContext) {
        let hrp = ctx.protocol_params().bech32_hrp();
        // The idea behind the following code is that we keep track of the deltas that are applied to each account that
        // is represented by an address.
        let mut balance_deltas: HashMap<Bech32Address, i128> = HashMap::new();

        // We first gather all tokens that have been moved to an individual address.
        for output in created {
            if let Some(a) = output.address() {
                *balance_deltas.entry(a.clone().to_bech32(hrp)).or_default() += output.amount() as i128;
            }
        }

        self.booked_amount += balance_deltas.values().sum::<i128>() as u64;

        // Afterwards, we subtract the tokens from that address to get the actual deltas of each account.
        for output in consumed {
            if let Some(a) = output.address() {
                *balance_deltas.entry(a.clone().to_bech32(hrp)).or_default() -= output.amount() as i128;
            }
        }

        // The number of transferred tokens is then the sum of all deltas.
        self.transferred_amount += balance_deltas.values().copied().map(|d| d.max(0) as u64).sum::<u64>();
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        std::mem::take(self)
    }
}
