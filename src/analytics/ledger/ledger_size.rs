// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{
    output::{Output, Rent},
    protocol::ProtocolParameters,
};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::ledger::{LedgerOutput, LedgerSpent},
};

trait LedgerSize {
    fn ledger_size(&self, protocol_params: &ProtocolParameters) -> LedgerSizeMeasurement;
}

impl LedgerSize for Output {
    fn ledger_size(&self, protocol_params: &ProtocolParameters) -> LedgerSizeMeasurement {
        LedgerSizeMeasurement {
            total_storage_cost: self.rent_cost(protocol_params.rent_structure()),
        }
    }
}

/// Ledger size statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct LedgerSizeMeasurement {
    pub(crate) total_storage_cost: u64,
}

impl LedgerSizeMeasurement {
    fn wrapping_add(&mut self, rhs: Self) {
        *self = Self {
            total_storage_cost: self.total_storage_cost.wrapping_add(rhs.total_storage_cost),
        }
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        *self = Self {
            total_storage_cost: self.total_storage_cost.wrapping_sub(rhs.total_storage_cost),
        }
    }
}

/// Measures the ledger size depending on current protocol parameters.
#[derive(Serialize, Deserialize)]
pub(crate) struct LedgerSizeAnalytics {
    measurement: LedgerSizeMeasurement,
}

impl LedgerSizeAnalytics {
    /// Set the protocol parameters for this analytic.
    pub(crate) fn init<'a>(
        protocol_params: &ProtocolParameters,
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
    ) -> Self {
        let mut measurement = LedgerSizeMeasurement::default();
        for output in unspent_outputs {
            measurement.wrapping_add(output.output().ledger_size(protocol_params));
        }
        Self { measurement }
    }
}

impl Analytics for LedgerSizeAnalytics {
    type Measurement = LedgerSizeMeasurement;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], ctx: &dyn AnalyticsContext) {
        for output in created {
            self.measurement
                .wrapping_add(output.output().ledger_size(ctx.protocol_parameters()));
        }
        for output in consumed.iter().map(|ledger_spent| &ledger_spent.output) {
            self.measurement
                .wrapping_sub(output.output().ledger_size(ctx.protocol_parameters()));
        }
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        self.measurement
    }
}
