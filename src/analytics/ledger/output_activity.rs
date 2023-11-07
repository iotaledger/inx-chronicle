// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use iota_sdk::types::block::{
    address::Address,
    output::{AccountId, AnchorId, DelegationId},
};

use super::*;

/// Nft activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub(crate) struct OutputActivityMeasurement {
    pub(crate) nft: NftActivityMeasurement,
    pub(crate) account: AccountActivityMeasurement,
    pub(crate) foundry: FoundryActivityMeasurement,
    pub(crate) anchor: AnchorActivityMeasurement,
    pub(crate) delegation: DelegationActivityMeasurement,
}

impl Analytics for OutputActivityMeasurement {
    type Measurement = Self;

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], _ctx: &dyn AnalyticsContext) {
        self.nft.handle_transaction(consumed, created);
        self.account.handle_transaction(consumed, created);
        self.foundry.handle_transaction(consumed, created);
        self.anchor.handle_transaction(consumed, created);
    }

    fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> Self::Measurement {
        std::mem::take(self)
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
        let map = |ledger_output: &LedgerOutput| {
            ledger_output
                .output
                .as_nft_opt()
                .map(|output| output.nft_id_non_null(&ledger_output.output_id))
        };

        let nft_inputs = consumed
            .iter()
            .map(|o| &o.output)
            .filter_map(map)
            .collect::<HashSet<_>>();

        let nft_outputs = created.iter().filter_map(map).collect::<HashSet<_>>();

        self.created_count += nft_outputs.difference(&nft_inputs).count();
        self.transferred_count += nft_outputs.intersection(&nft_inputs).count();
        self.destroyed_count += nft_inputs.difference(&nft_outputs).count();
    }
}

/// Account activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct AccountActivityMeasurement {
    pub(crate) created_count: usize,
    pub(crate) destroyed_count: usize,
}

struct AccountData {
    account_id: AccountId,
}

impl std::cmp::PartialEq for AccountData {
    fn eq(&self, other: &Self) -> bool {
        self.account_id == other.account_id
    }
}

impl std::cmp::Eq for AccountData {}

impl std::hash::Hash for AccountData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.account_id.hash(state);
    }
}

impl AccountActivityMeasurement {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let map = |ledger_output: &LedgerOutput| {
            ledger_output.output.as_account_opt().map(|output| AccountData {
                account_id: output.account_id_non_null(&ledger_output.output_id),
            })
        };

        let account_inputs = consumed
            .iter()
            .map(|o| &o.output)
            .filter_map(map)
            .collect::<HashSet<_>>();

        let account_outputs = created.iter().filter_map(map).collect::<HashSet<_>>();

        self.created_count += account_outputs.difference(&account_inputs).count();
        self.destroyed_count += account_inputs.difference(&account_outputs).count();
    }
}

/// Anchor activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct AnchorActivityMeasurement {
    pub(crate) created_count: usize,
    pub(crate) governor_changed_count: usize,
    pub(crate) state_changed_count: usize,
    pub(crate) destroyed_count: usize,
}

struct AnchorData {
    anchor_id: AnchorId,
    governor_address: Address,
    state_index: u32,
}

impl std::cmp::PartialEq for AnchorData {
    fn eq(&self, other: &Self) -> bool {
        self.anchor_id == other.anchor_id
    }
}

impl std::cmp::Eq for AnchorData {}

impl std::hash::Hash for AnchorData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.anchor_id.hash(state);
    }
}

impl AnchorActivityMeasurement {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let map = |ledger_output: &LedgerOutput| {
            ledger_output.output.as_anchor_opt().map(|output| AnchorData {
                anchor_id: output.anchor_id_non_null(&ledger_output.output_id),
                governor_address: output.governor_address().clone(),
                state_index: output.state_index(),
            })
        };

        let anchor_inputs = consumed
            .iter()
            .map(|o| &o.output)
            .filter_map(map)
            .collect::<HashSet<_>>();

        let anchor_outputs = created.iter().filter_map(map).collect::<HashSet<_>>();

        self.created_count += anchor_outputs.difference(&anchor_inputs).count();
        self.destroyed_count += anchor_inputs.difference(&anchor_outputs).count();

        for anchor_data in anchor_outputs.intersection(&anchor_inputs) {
            // Unwraps: cannot fail because we iterate the intersection so those elements must exist
            let input_state_index = anchor_inputs.get(anchor_data).unwrap().state_index;
            let output_state_index = anchor_outputs.get(anchor_data).unwrap().state_index;
            if output_state_index != input_state_index {
                self.state_changed_count += 1;
            }
            let input_governor_address = &anchor_inputs.get(anchor_data).unwrap().governor_address;
            let output_governor_address = &anchor_outputs.get(anchor_data).unwrap().governor_address;
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
        let map = |ledger_output: &LedgerOutput| ledger_output.output.as_foundry_opt().map(|output| output.id());

        let foundry_inputs = consumed
            .iter()
            .map(|o| &o.output)
            .filter_map(map)
            .collect::<HashSet<_>>();

        let foundry_outputs = created.iter().filter_map(map).collect::<HashSet<_>>();

        self.created_count += foundry_outputs.difference(&foundry_inputs).count();
        self.transferred_count += foundry_outputs.intersection(&foundry_inputs).count();
        self.destroyed_count += foundry_inputs.difference(&foundry_outputs).count();
    }
}

/// Delegation activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct DelegationActivityMeasurement {
    pub(crate) created_count: usize,
    pub(crate) destroyed_count: usize,
}

struct DelegationData {
    delegation_id: DelegationId,
}

impl std::cmp::PartialEq for DelegationData {
    fn eq(&self, other: &Self) -> bool {
        self.delegation_id == other.delegation_id
    }
}

impl std::cmp::Eq for DelegationData {}

impl std::hash::Hash for DelegationData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.delegation_id.hash(state);
    }
}

impl DelegationActivityMeasurement {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let map = |ledger_output: &LedgerOutput| {
            ledger_output.output.as_delegation_opt().map(|output| DelegationData {
                delegation_id: output.delegation_id_non_null(&ledger_output.output_id),
            })
        };
        let delegation_inputs = consumed
            .iter()
            .map(|o| &o.output)
            .filter_map(map)
            .collect::<HashSet<_>>();

        let delegation_outputs = created.iter().filter_map(map).collect::<HashSet<_>>();

        self.created_count += delegation_outputs.difference(&delegation_inputs).count();
        self.destroyed_count += delegation_inputs.difference(&delegation_outputs).count();
    }
}
