// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use derive_more::{AddAssign, SubAssign};

use super::TransactionAnalytics;
use crate::{
    db::collections::analytics::{
        AliasActivityAnalyticsResult, FoundryActivityAnalyticsResult, NftActivityAnalyticsResult,
        OutputActivityAnalyticsResult,
    },
    types::{
        ledger::{LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
        stardust::block::{
            output::{AliasId, NftId},
            Address, Output,
        },
    },
};

/// Nft activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
pub struct OutputActivityAnalytics {
    nft: NftActivityAnalytics,
    alias: AliasActivityAnalytics,
    foundry: FoundryActivityAnalytics,
}

impl TransactionAnalytics for OutputActivityAnalytics {
    type Measurement = OutputActivityAnalyticsResult;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {
        *self = Self::default();
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        self.nft.handle_transaction(consumed, created);
        self.alias.handle_transaction(consumed, created);
        self.foundry.handle_transaction(consumed, created);
    }

    fn end_milestone(&mut self, at: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(OutputActivityAnalyticsResult {
            nft: self.nft.end_milestone(at).unwrap_or_default(),
            alias: self.alias.end_milestone(at).unwrap_or_default(),
            foundry: self.foundry.end_milestone(at).unwrap_or_default(),
        })
    }
}

/// Nft activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
struct NftActivityAnalytics {
    created_count: usize,
    transferred_count: usize,
    destroyed_count: usize,
}

impl TransactionAnalytics for NftActivityAnalytics {
    type Measurement = NftActivityAnalyticsResult;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {
        *self = Self::default();
    }

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

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(NftActivityAnalyticsResult {
            created_count: self.created_count as u64,
            transferred_count: self.transferred_count as u64,
            destroyed_count: self.destroyed_count as u64,
        })
    }
}

/// Alias activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
struct AliasActivityAnalytics {
    created_count: usize,
    governor_changed_count: usize,
    state_changed_count: usize,
    destroyed_count: usize,
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
    type Measurement = AliasActivityAnalyticsResult;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {}

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

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(AliasActivityAnalyticsResult {
            created_count: self.created_count as u64,
            governor_changed_count: self.governor_changed_count as u64,
            state_changed_count: self.state_changed_count as u64,
            destroyed_count: self.destroyed_count as u64,
        })
    }
}

/// Nft activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, AddAssign, SubAssign)]
struct FoundryActivityAnalytics {
    created_count: usize,
    transferred_count: usize,
    destroyed_count: usize,
}

impl TransactionAnalytics for FoundryActivityAnalytics {
    type Measurement = FoundryActivityAnalyticsResult;

    fn begin_milestone(&mut self, _: MilestoneIndexTimestamp) {
        *self = Self::default();
    }

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

    fn end_milestone(&mut self, _: MilestoneIndexTimestamp) -> Option<Self::Measurement> {
        Some(FoundryActivityAnalyticsResult {
            created_count: self.created_count as u64,
            transferred_count: self.transferred_count as u64,
            destroyed_count: self.destroyed_count as u64,
        })
    }
}
