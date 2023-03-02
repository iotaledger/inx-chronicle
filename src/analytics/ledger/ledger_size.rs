// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::model::{ledger::RentStructureBytes, ProtocolParameters, TryFromWithContext};

trait LedgerSize {
    fn ledger_size(&self, protocol_params: &ProtocolParameters) -> LedgerSizeMeasurement;
}

impl LedgerSize for Output {
    fn ledger_size(&self, protocol_params: &ProtocolParameters) -> LedgerSizeMeasurement {
        // Unwrap: acceptable risk
        let protocol_params = iota_types::block::protocol::ProtocolParameters::try_from(protocol_params.clone())
            .expect("protocol parameters conversion error");
        let output = iota_types::block::output::Output::try_from_with_context(&protocol_params, self.clone()).unwrap();
        let rent_bytes = RentStructureBytes::compute(&output);
        LedgerSizeMeasurement {
            total_storage_deposit_amount: iota_types::block::output::Rent::rent_cost(
                &output,
                protocol_params.rent_structure(),
            )
            .into(),
            total_key_bytes: rent_bytes.num_key_bytes,
            total_data_bytes: rent_bytes.num_data_bytes,
        }
    }
}

/// Ledger size statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct LedgerSizeMeasurement {
    pub(crate) total_key_bytes: u64,
    pub(crate) total_data_bytes: u64,
    pub(crate) total_storage_deposit_amount: TokenAmount,
}

impl LedgerSizeMeasurement {
    fn wrapping_add(&mut self, rhs: Self) {
        *self = Self {
            total_key_bytes: self.total_key_bytes.wrapping_add(rhs.total_key_bytes),
            total_data_bytes: self.total_data_bytes.wrapping_add(rhs.total_data_bytes),
            total_storage_deposit_amount: TokenAmount(
                self.total_storage_deposit_amount
                    .0
                    .wrapping_add(rhs.total_storage_deposit_amount.0),
            ),
        }
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        *self = Self {
            total_key_bytes: self.total_key_bytes.wrapping_sub(rhs.total_key_bytes),
            total_data_bytes: self.total_data_bytes.wrapping_sub(rhs.total_data_bytes),
            total_storage_deposit_amount: TokenAmount(
                self.total_storage_deposit_amount
                    .0
                    .wrapping_sub(rhs.total_storage_deposit_amount.0),
            ),
        }
    }
}

/// Measures the ledger size depending on current protocol parameters.
#[derive(Serialize, Deserialize)]
pub(crate) struct LedgerSizeAnalytics {
    protocol_params: ProtocolParameters,
    measurement: LedgerSizeMeasurement,
}

impl LedgerSizeAnalytics {
    /// Set the protocol parameters for this analytic.
    pub(crate) fn init<'a>(
        protocol_params: ProtocolParameters,
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
    ) -> Self {
        let mut measurement = LedgerSizeMeasurement::default();
        for output in unspent_outputs {
            measurement.wrapping_add(output.output.ledger_size(&protocol_params));
        }
        Self {
            protocol_params,
            measurement,
        }
    }
}

impl Analytics for LedgerSizeAnalytics {
    type Measurement = LedgerSizeMeasurement;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        for output in created {
            self.measurement
                .wrapping_add(output.output.ledger_size(&self.protocol_params));
        }
        for output in consumed.iter().map(|ledger_spent| &ledger_spent.output) {
            self.measurement
                .wrapping_sub(output.output.ledger_size(&self.protocol_params));
        }
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        self.measurement
    }
}
