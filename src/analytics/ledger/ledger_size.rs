// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use derive_more::{AddAssign, SubAssign};
use iota_types::block::output::{self as iota, Rent};

use super::LedgerUpdateAnalytics;
use crate::{
    inx::LedgerUpdateMarker,
    types::{
        ledger::{LedgerOutput, LedgerSpent},
        tangle::ProtocolParameters,
    },
};

#[derive(Clone, Debug, Default, AddAssign, SubAssign)]
pub struct LedgerSizeStatistic {
    pub total_storage_deposit_value: u64,
    pub total_key_bytes: u64,
    pub total_data_bytes: u64,
}

#[derive(Debug)]
pub struct LedgerSizeAnalytics {
    rent_structure: iota::RentStructure,
    stats: LedgerSizeStatistic,
}

impl LedgerSizeAnalytics {
    pub fn new(config: ProtocolParameters) -> Self {
        Self {
            rent_structure: iota::RentStructure::from(config.rent_structure),
            stats: LedgerSizeStatistic::default(),
        }
    }

    pub fn handle_unspent_output(&mut self, output: &LedgerOutput) {
        self.handle_created(output);
    }
}

impl LedgerUpdateAnalytics for LedgerSizeAnalytics {
    type Measurement = LedgerSizeStatistic;

    fn begin(&mut self, _marker: LedgerUpdateMarker) {}

    fn handle_created(&mut self, output: &LedgerOutput) {
        self.stats += LedgerSizeStatistic {
            total_storage_deposit_value: output.rent_structure.rent_cost(&self.rent_structure),
            total_data_bytes: output.rent_structure.num_data_bytes,
            total_key_bytes: output.rent_structure.num_key_bytes,
        }
    }

    fn handle_consumed(&mut self, spent: &LedgerSpent) {
        self.stats -= LedgerSizeStatistic {
            total_storage_deposit_value: spent.output.rent_structure.rent_cost(&self.rent_structure),
            total_data_bytes: spent.output.rent_structure.num_data_bytes,
            total_key_bytes: spent.output.rent_structure.num_key_bytes,
        }
    }

    fn flush(&mut self, _marker: LedgerUpdateMarker) -> Option<Self::Measurement> {
        Some(self.stats.clone())
    }
}
