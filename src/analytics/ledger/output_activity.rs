// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use derive_more::{AddAssign, SubAssign};

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
    stardust::block::{
        output::{AliasId, NftId},
        Address, Output,
    },
};

/// Nft activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
pub struct NftActivityMeasurement {
    pub created_count: u64,
    pub transferred_count: u64,
    pub destroyed_count: u64,
}

/// Measures the ledger Nft activity.
#[derive(Debug)]
pub struct NftActivityAnalytics {
    measurement: NftActivityMeasurement,
}

impl TransactionAnalytics for NftActivityAnalytics {
    type Measurement = NftActivityMeasurement;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {
        self.measurement = NftActivityMeasurement::default();
    }

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        let nft_inputs = inputs
            .iter()
            .filter_map(|ledger_spent| {
                if let Output::Nft(nft_output) = &ledger_spent.output.output {
                    if nft_output.nft_id == NftId::implicit() {
                        // Convert implicit ids to explicit ids to make all nfts comparable
                        Some(NftId::from(ledger_spent.output.output_id))
                    } else {
                        Some(nft_output.nft_id)
                    }
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        let nft_outputs = outputs
            .iter()
            .filter_map(|ledger_output| {
                if let Output::Nft(nft_output) = &ledger_output.output {
                    if nft_output.nft_id == NftId::implicit() {
                        // Convert implicit ids to explicit ids to make all nfts comparable
                        Some(NftId::from(ledger_output.output_id))
                    } else {
                        Some(nft_output.nft_id)
                    }
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        self.measurement.created_count += nft_outputs.difference(&nft_inputs).count() as u64;
        self.measurement.transferred_count += nft_outputs.intersection(&nft_inputs).count() as u64;
        self.measurement.destroyed_count += nft_inputs.difference(&nft_outputs).count() as u64;
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(self.measurement)
    }
}

/// Alias activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
pub struct AliasActivityMeasurement {
    pub created_count: u64,
    pub governor_changed_count: u64,
    pub state_changed_count: u64,
    pub destroyed_count: u64,
}

/// Measures the ledger Alias activity.
pub struct AliasActivityAnalytics {
    measurement: AliasActivityMeasurement,
}

struct AliasData {
    alias_id: AliasId,
    governor_address: Address,
    state_index: u32,
}

impl std::cmp::PartialEq for AliasData {
    fn eq(&self, other: &Self) -> bool {
        self.alias_id == other.alias_id
    }
}

impl std::cmp::Eq for AliasData {}

impl std::hash::Hash for AliasData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.alias_id.hash(state);
    }
}

impl TransactionAnalytics for AliasActivityAnalytics {
    type Measurement = AliasActivityMeasurement;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

    fn handle_transaction(&mut self, inputs: &[LedgerSpent], outputs: &[LedgerOutput]) {
        let alias_inputs = inputs
            .iter()
            .filter_map(|ledger_spent| {
                if let Output::Alias(alias_output) = &ledger_spent.output.output {
                    let alias_id = if alias_output.alias_id == AliasId::implicit() {
                        // Convert implicit ids to explicit ids to make all aliases comparable
                        AliasId::from(ledger_spent.output.output_id)
                    } else {
                        alias_output.alias_id
                    };
                    Some(AliasData {
                        alias_id,
                        governor_address: alias_output.governor_address_unlock_condition.address,
                        state_index: alias_output.state_index,
                    })
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        let alias_outputs = outputs
            .iter()
            .filter_map(|ledger_output| {
                if let Output::Alias(alias_output) = &ledger_output.output {
                    let alias_id = if alias_output.alias_id == AliasId::implicit() {
                        // Convert implicit ids to explicit ids to make all aliases comparable
                        AliasId::from(ledger_output.output_id)
                    } else {
                        alias_output.alias_id
                    };

                    Some(AliasData {
                        alias_id,
                        governor_address: alias_output.governor_address_unlock_condition.address,
                        state_index: alias_output.state_index,
                    })
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        self.measurement.created_count += alias_outputs.difference(&alias_inputs).count() as u64;
        self.measurement.destroyed_count += alias_inputs.difference(&alias_outputs).count() as u64;

        for alias_data in alias_outputs.intersection(&alias_inputs) {
            // Unwraps: cannot fail because we iterate the intersection so those elements must exist
            let input_state_index = alias_inputs.get(alias_data).unwrap().state_index;
            let output_state_index = alias_outputs.get(alias_data).unwrap().state_index;
            if output_state_index != input_state_index {
                self.measurement.state_changed_count += 1;
            }
            let input_governor_address = alias_inputs.get(alias_data).unwrap().governor_address;
            let output_governor_address = alias_outputs.get(alias_data).unwrap().governor_address;
            if output_governor_address != input_governor_address {
                self.measurement.governor_changed_count += 1;
            }
        }
    }

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(self.measurement)
    }
}
