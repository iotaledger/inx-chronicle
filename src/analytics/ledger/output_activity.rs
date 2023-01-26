// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use derive_more::{AddAssign, SubAssign};

use super::TransactionAnalytics;
use crate::types::{
    ledger::{LedgerOutput, LedgerSpent},
    stardust::block::{
        output::{AliasId, NftId},
        Output,
    },
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
            .filter_map(|ledger_spent| {
                if let Output::Nft(nft_output) = &ledger_spent.output.output {
                    if nft_output.nft_id == NftId::implicit() {
                        // TODO: handle unwrap
                        let output_id: iota_types::block::output::OutputId =
                            ledger_spent.output.output_id.try_into().unwrap();
                        let nft_id: NftId = iota_types::block::output::NftId::null()
                            .or_from_output_id(&output_id)
                            .into();
                        Some(nft_id)
                    } else {
                        Some(nft_output.nft_id)
                    }
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        let outputs = outputs
            .iter()
            .filter_map(|ledger_output| {
                if let Output::Nft(nft_output) = &ledger_output.output {
                    if nft_output.nft_id == NftId::implicit() {
                        // TODO: handle unwrap
                        let output_id: iota_types::block::output::OutputId =
                            ledger_output.output_id.try_into().unwrap();
                        let nft_id: NftId = iota_types::block::output::NftId::null()
                            .or_from_output_id(&output_id)
                            .into();
                        Some(nft_id)
                    } else {
                        Some(nft_output.nft_id)
                    }
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
                if let Output::Alias(alias_output) = &ledger_spent.output.output {
                    if alias_output.alias_id == AliasId::implicit() {
                        // TODO: handle unwrap
                        let output_id: iota_types::block::output::OutputId =
                            ledger_spent.output.output_id.try_into().unwrap();
                        let alias_id: AliasId = iota_types::block::output::AliasId::null()
                            .or_from_output_id(&output_id)
                            .into();
                        Some(alias_id)
                    } else {
                        Some(alias_output.alias_id)
                    }
                    // TODO
                    // alias_output.governor_address_unlock_condition.address
                    // alias_output.state_index
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        let outputs = outputs
            .iter()
            .filter_map(|ledger_output| {
                if let Output::Alias(alias_output) = &ledger_output.output {
                    if alias_output.alias_id == AliasId::implicit() {
                        // TODO: handle unwrap
                        let output_id: iota_types::block::output::OutputId =
                            ledger_output.output_id.try_into().unwrap();
                        let alias_id: AliasId = iota_types::block::output::AliasId::null()
                            .or_from_output_id(&output_id)
                            .into();
                        Some(alias_id)
                    } else {
                        Some(alias_output.alias_id)
                    }
                    // TODO
                    // alias_output.governor_address_unlock_condition.address
                    // alias_output.state_index
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
