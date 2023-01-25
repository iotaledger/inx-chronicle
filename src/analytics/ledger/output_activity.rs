// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use derive_more::{AddAssign, SubAssign};

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent},
    tangle::MilestoneIndex, stardust::block::Output,
};

#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
pub struct NftActivityMeasurement {
    pub created_count: u64,
    pub transferred_count: u64,
    pub destroyed_count: u64,
}

#[derive(Debug)]
pub struct NftActivityAnalytics {
    measurement: NftActivityMeasurement,
}

impl TransactionAnalytics for NftActivityAnalytics {
    type Measurement = NftActivityMeasurement;

    fn begin_milestone(&mut self, _: MilestoneIndex) {
        self.measurement = NftActivityMeasurement::default();
    }

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        let inputs = inputs
            .iter()
            .filter_map(|ledger_spent| {
                if matches!(ledger_spent.output.output, Output::Nft(_)) {
                    Some(ledger_spent.output.output_id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        let outputs = outputs
            .iter()
            .filter_map(|ledger_output| {
                if matches!(ledger_output.output, Output::Nft(_)) {
                    Some(ledger_output.output_id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        self.measurement.created_count += outputs.difference(&inputs).count() as u64;
        self.measurement.transferred_count += outputs.intersection(&inputs).count() as u64;
        self.measurement.destroyed_count += inputs.difference(&outputs).count() as u64;
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
pub struct AliasActivityMeasurement {
    pub created_count: u64,
    pub governor_changed_count: u64,
    pub state_changed_count: u64,
    pub destroyed_count: u64,
}

pub struct AliasActivityAnalytics {
    measurement: AliasActivityMeasurement,
}

impl TransactionAnalytics for AliasActivityAnalytics {
    type Measurement = AliasActivityMeasurement;

    fn begin_milestone(&mut self, _: MilestoneIndex) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        let inputs = inputs
            .iter()
            .filter_map(|ledger_spent| {
                if matches!(ledger_spent.output.output, Output::Alias(_)) {
                    Some(ledger_spent.output.output_id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        let outputs = outputs
            .iter()
            .filter_map(|ledger_output| {
                if matches!(ledger_output.output, Output::Alias(_)) {
                    Some(ledger_output.output_id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        self.measurement.created_count += outputs.difference(&inputs).count() as u64;
        self.measurement.destroyed_count += inputs.difference(&outputs).count() as u64;
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement)
    }
}
