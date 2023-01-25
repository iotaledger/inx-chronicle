// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use derive_more::{AddAssign, SubAssign};

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent},
    tangle::MilestoneIndex,
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
            .map(|ledger_spent| ledger_spent.output.output_id)
            .collect::<HashSet<_>>();

        let outputs = outputs
            .iter()
            .map(|ledger_output| ledger_output.output_id)
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
            .map(|ledger_spent| ledger_spent.output.output_id)
            .collect::<HashSet<_>>();

        let outputs = outputs
            .iter()
            .map(|ledger_output| ledger_output.output_id)
            .collect::<HashSet<_>>();

        self.measurement.created_count += outputs.difference(&inputs).count() as u64;
        self.measurement.destroyed_count += inputs.difference(&outputs).count() as u64;
    }

    fn end_milestone(&mut self, _: MilestoneIndex) -> Option<Self::Measurement> {
        Some(self.measurement)
    }
}
