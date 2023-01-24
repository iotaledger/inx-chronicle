// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use derive_more::{AddAssign, SubAssign};

use super::BlockAnalytics;
use crate::types::{
    ledger::{BlockMetadata, RentStructureBytes, LedgerInclusionState},
    stardust::block::{Block, Output, Payload, payload::TransactionEssence},
    tangle::{MilestoneIndex, ProtocolParameters}, context::TryFromWithContext,
};
trait LedgerSize {
    fn ledger_size(&self, protocol_params: &ProtocolParameters) -> LedgerSizeMeasurement;
}

impl LedgerSize for Output {
    fn ledger_size(&self, protocol_params: &ProtocolParameters) -> LedgerSizeMeasurement {
        let config = iota_types::block::protocol::ProtocolParameters::try_from(protocol_params.clone()).unwrap();
        let output = iota_types::block::output::Output::try_from_with_context(&config, self.clone()).unwrap();
        let rent_bytes = RentStructureBytes::compute(&output);
        LedgerSizeMeasurement {
            total_storage_deposit_value: iota_types::block::output::Rent::rent_cost(&output, config.rent_structure()),
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

#[derive(Debug, Default)]
pub struct LedgerSizeAnalytics {
    protocol_params: Option<ProtocolParameters>,
    measurement: LedgerSizeMeasurement,
}

impl BlockAnalytics for LedgerSizeAnalytics {
    type Measurement = LedgerSizeMeasurement;
    type Context = ProtocolParameters;

    fn begin_milestone(&mut self, ctx: Self::Context) {
        self.protocol_params = Some(ctx);
    }

    fn handle_block(&mut self, block: &Block, block_metadata: &BlockMetadata, inputs: &Option<Vec<Output>>) {
        if block_metadata.inclusion_state == LedgerInclusionState::Included {
            let protocol_params = self.protocol_params.as_ref().unwrap();
            if let Some(payload) = block.payload.as_ref() {
                match payload {
                    Payload::Transaction(txn) => {
                        let TransactionEssence::Regular { outputs, .. } = &txn.essence;
                        for output in outputs.iter() {
                            self.measurement += output.ledger_size(protocol_params);
                        }
                    }
                    _ => (),
                }
            }
            if let Some(inputs) = inputs {
                for input in inputs {
                    self.measurement -= input.ledger_size(protocol_params);
                }
            }
        }
    }

    fn end_milestone(&mut self, index: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement)
    }
}
