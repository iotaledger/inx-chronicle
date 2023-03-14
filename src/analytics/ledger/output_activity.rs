// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use influxdb::WriteQuery;

use super::*;
use crate::{
    analytics::measurement::Measurement,
    model::utxo::{Address, AliasId, NftId},
};

/// Nft activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub(crate) struct OutputActivityMeasurement {
    pub(crate) nft: NftActivityMeasurement,
    pub(crate) alias: AliasActivityMeasurement,
    pub(crate) foundry: FoundryActivityMeasurement,
}

impl Analytics for OutputActivityMeasurement {
    type Measurement = Self;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        self.nft.handle_transaction(consumed, created);
        self.alias.handle_transaction(consumed, created);
        self.foundry.handle_transaction(consumed, created);
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        std::mem::take(self)
    }
}

impl Measurement for OutputActivityMeasurement {
    const NAME: &'static str = "stardust_output_activity";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("alias_created_count", self.alias.created_count as u64)
            .add_field("alias_state_changed_count", self.alias.state_changed_count as u64)
            .add_field("alias_governor_changed_count", self.alias.governor_changed_count as u64)
            .add_field("alias_destroyed_count", self.alias.destroyed_count as u64)
            .add_field("nft_created_count", self.nft.created_count as u64)
            .add_field("nft_transferred_count", self.nft.transferred_count as u64)
            .add_field("nft_destroyed_count", self.nft.destroyed_count as u64)
            .add_field("foundry_created_count", self.foundry.created_count as u64)
            .add_field("foundry_transferred_count", self.foundry.transferred_count as u64)
            .add_field("foundry_destroyed_count", self.foundry.destroyed_count as u64)
    }
}

/// Nft activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct NftActivityMeasurement {
    pub(crate) created_count: usize,
    pub(crate) transferred_count: usize,
    pub(crate) destroyed_count: usize,
}

impl NftActivityMeasurement {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let nft_inputs = consumed
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

        let nft_outputs = created
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

        self.created_count += nft_outputs.difference(&nft_inputs).count();
        self.transferred_count += nft_outputs.intersection(&nft_inputs).count();
        self.destroyed_count += nft_inputs.difference(&nft_outputs).count();
    }
}

/// Alias activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct AliasActivityMeasurement {
    pub(crate) created_count: usize,
    pub(crate) governor_changed_count: usize,
    pub(crate) state_changed_count: usize,
    pub(crate) destroyed_count: usize,
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

impl AliasActivityMeasurement {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let alias_inputs = consumed
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

        let alias_outputs = created
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

        self.created_count += alias_outputs.difference(&alias_inputs).count();
        self.destroyed_count += alias_inputs.difference(&alias_outputs).count();

        for alias_data in alias_outputs.intersection(&alias_inputs) {
            // Unwraps: cannot fail because we iterate the intersection so those elements must exist
            let input_state_index = alias_inputs.get(alias_data).unwrap().state_index;
            let output_state_index = alias_outputs.get(alias_data).unwrap().state_index;
            if output_state_index != input_state_index {
                self.state_changed_count += 1;
            }
            let input_governor_address = alias_inputs.get(alias_data).unwrap().governor_address;
            let output_governor_address = alias_outputs.get(alias_data).unwrap().governor_address;
            if output_governor_address != input_governor_address {
                self.governor_changed_count += 1;
            }
        }
    }
}

/// Nft activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct FoundryActivityMeasurement {
    pub(crate) created_count: usize,
    pub(crate) transferred_count: usize,
    pub(crate) destroyed_count: usize,
}

impl FoundryActivityMeasurement {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let foundry_inputs = consumed
            .iter()
            .filter_map(|ledger_spent| {
                if let Output::Foundry(foundry_output) = &ledger_spent.output.output {
                    Some(foundry_output.foundry_id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        let foundry_outputs = created
            .iter()
            .filter_map(|ledger_output| {
                if let Output::Foundry(foundry_output) = &ledger_output.output {
                    Some(foundry_output.foundry_id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        self.created_count += foundry_outputs.difference(&foundry_inputs).count();
        self.transferred_count += foundry_outputs.intersection(&foundry_inputs).count();
        self.destroyed_count += foundry_inputs.difference(&foundry_outputs).count();
    }
}
