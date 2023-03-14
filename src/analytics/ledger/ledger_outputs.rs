// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use influxdb::WriteQuery;

use super::*;
use crate::analytics::measurement::Measurement;

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct LedgerOutputMeasurement {
    pub(crate) alias: CountAndAmount,
    pub(crate) basic: CountAndAmount,
    pub(crate) nft: CountAndAmount,
    pub(crate) foundry: CountAndAmount,
    pub(crate) treasury: CountAndAmount,
}

impl LedgerOutputMeasurement {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) fn init<'a>(unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>) -> Self {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            match output.output {
                Output::Alias(_) => measurement.alias.add_output(output),
                Output::Basic(_) => measurement.basic.add_output(output),
                Output::Nft(_) => measurement.nft.add_output(output),
                Output::Foundry(_) => measurement.foundry.add_output(output),
                Output::Treasury(_) => measurement.treasury.add_output(output),
            }
        }
        measurement
    }

    fn wrapping_add(&mut self, rhs: Self) {
        self.alias.wrapping_add(rhs.alias);
        self.basic.wrapping_add(rhs.basic);
        self.nft.wrapping_add(rhs.nft);
        self.foundry.wrapping_add(rhs.foundry);
        self.treasury.wrapping_add(rhs.treasury);
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        self.alias.wrapping_sub(rhs.alias);
        self.basic.wrapping_sub(rhs.basic);
        self.nft.wrapping_sub(rhs.nft);
        self.foundry.wrapping_sub(rhs.foundry);
        self.treasury.wrapping_sub(rhs.treasury);
    }
}

impl Analytics for LedgerOutputMeasurement {
    type Measurement = Self;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        let consumed = Self::init(consumed.iter().map(|input| &input.output));
        let created = Self::init(created);

        self.wrapping_sub(consumed);
        self.wrapping_add(created);
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        *self
    }
}

impl Measurement for LedgerOutputMeasurement {
    const NAME: &'static str = "stardust_ledger_outputs";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("basic_count", self.basic.count as u64)
            .add_field("basic_amount", self.basic.amount.0)
            .add_field("alias_count", self.alias.count as u64)
            .add_field("alias_amount", self.alias.amount.0)
            .add_field("foundry_count", self.foundry.count as u64)
            .add_field("foundry_amount", self.foundry.amount.0)
            .add_field("nft_count", self.nft.count as u64)
            .add_field("nft_amount", self.nft.amount.0)
            .add_field("treasury_count", self.treasury.count as u64)
            .add_field("treasury_amount", self.treasury.amount.0)
    }
}
