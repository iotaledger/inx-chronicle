// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use derive_more::{AddAssign, SubAssign};

use super::TransactionAnalytics;
use crate::types::{
    context::TryFromWithContext,
    ledger::{LedgerOutput, LedgerSpent, RentStructureBytes},
    stardust::block::Output,
    tangle::{MilestoneIndex, ProtocolParameters},
};
trait LedgerSize {
    fn ledger_size(&self, protocol_params: &ProtocolParameters) -> LedgerSizeMeasurement;
}

impl LedgerSize for Output {
    fn ledger_size(&self, protocol_params: &ProtocolParameters) -> LedgerSizeMeasurement {
        // What do we do if this fails?
        let protocol_params =
            iota_types::block::protocol::ProtocolParameters::try_from(protocol_params.clone()).unwrap();
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

#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
pub struct LedgerSizeMeasurement {
    pub total_key_bytes: u64,
    pub total_data_bytes: u64,
    pub total_storage_deposit_value: u64,
}

pub struct LedgerSizeAnalytics {
    protocol_params: ProtocolParameters,
    measurement: LedgerSizeMeasurement,
}

impl LedgerSizeAnalytics {
    // FIXME: temporarily allowed
    #[allow(dead_code)]
    pub fn with_protocol_parameters(protocol_params: ProtocolParameters) -> Self {
        Self {
            protocol_params,
            measurement: LedgerSizeMeasurement::default(),
        }
    }
}

impl TransactionAnalytics for LedgerSizeAnalytics {
    type Measurement = LedgerSizeMeasurement;

    fn begin_milestone(&mut self, _: MilestoneIndex) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        for output in outputs {
            self.measurement += output.output.ledger_size(&self.protocol_params);
        }
        for input in inputs.iter().map(|ledger_spent| &ledger_spent.output) {
            self.measurement -= input.output.ledger_size(&self.protocol_params);
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement)
    }
}
