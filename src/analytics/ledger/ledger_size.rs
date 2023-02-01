// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use derive_more::{AddAssign, SubAssign};

use super::TransactionAnalytics;
use crate::types::{
    context::TryFromWithContext,
    ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp, RentStructureBytes},
    stardust::block::Output,
    tangle::ProtocolParameters,
};
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
            total_storage_deposit_value: iota_types::block::output::Rent::rent_cost(
                &output,
                protocol_params.rent_structure(),
            ),
            total_key_bytes: rent_bytes.num_key_bytes,
            total_data_bytes: rent_bytes.num_data_bytes,
        }
    }
}

/// Ledger size statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
pub struct LedgerSizeMeasurement {
    pub total_key_bytes: u64,
    pub total_data_bytes: u64,
    pub total_storage_deposit_value: u64,
}

/// Measures the ledger size depending on current protocol parameters.
pub struct LedgerSizeAnalytics {
    protocol_params: ProtocolParameters,
    measurement: LedgerSizeMeasurement,
}

impl LedgerSizeAnalytics {
    /// Set the protocol parameters for this analytic.
    pub fn init<'a>(
        protocol_params: ProtocolParameters,
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
    ) -> Self {
        let mut measurement = LedgerSizeMeasurement::default();
        for output in unspent_outputs {
            measurement += output.output.ledger_size(&protocol_params);
        }
        Self {
            protocol_params,
            measurement,
        }
    }
}

impl TransactionAnalytics for LedgerSizeAnalytics {
    type Measurement = LedgerSizeMeasurement;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        for output in created {
            self.measurement += output.output.ledger_size(&self.protocol_params);
        }
        for input in consumed.iter().map(|ledger_spent| &ledger_spent.output) {
            self.measurement -= input.output.ledger_size(&self.protocol_params);
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(self.measurement)
    }
}
