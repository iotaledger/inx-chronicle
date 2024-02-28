// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use iota_sdk::types::block::{
    address::Address,
    output::{AccountId, AccountOutput, AnchorId},
    payload::SignedTransactionPayload,
};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    model::ledger::{LedgerOutput, LedgerSpent},
};

/// Nft activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub(crate) struct OutputActivityMeasurement {
    pub(crate) nft: NftActivityMeasurement,
    pub(crate) account: AccountActivityMeasurement,
    pub(crate) anchor: AnchorActivityMeasurement,
    pub(crate) foundry: FoundryActivityMeasurement,
    pub(crate) delegation: DelegationActivityMeasurement,
    pub(crate) native_token: NativeTokenActivityMeasurement,
}

#[async_trait::async_trait]
impl Analytics for OutputActivityMeasurement {
    type Measurement = Self;

    async fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        _ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        self.nft.handle_transaction(consumed, created);
        self.account.handle_transaction(consumed, created);
        self.anchor.handle_transaction(consumed, created);
        self.foundry.handle_transaction(consumed, created);
        self.delegation.handle_transaction(consumed, created);
        self.native_token.handle_transaction(consumed, created);

        Ok(())
    }

    async fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        Ok(std::mem::take(self))
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
                .output()
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
    pub(crate) transferred_count: usize,
    pub(crate) block_issuer_key_rotated: usize,
    pub(crate) destroyed_count: usize,
}

impl AccountActivityMeasurement {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        fn map(ledger_output: &LedgerOutput) -> Option<(AccountId, &AccountOutput)> {
            ledger_output
                .output()
                .as_account_opt()
                .map(|output| (output.account_id_non_null(&ledger_output.output_id), output))
        }

        let account_inputs = consumed
            .iter()
            .map(|o| &o.output)
            .filter_map(map)
            .collect::<HashMap<_, _>>();

        let account_outputs = created.iter().filter_map(map).collect::<HashMap<_, _>>();

        self.created_count += account_outputs.difference_count(&account_inputs);
        self.transferred_count += account_outputs.intersection_count(&account_inputs);
        self.destroyed_count += account_inputs.difference_count(&account_outputs);
        for (account_id, output_feature) in account_outputs
            .into_iter()
            .filter_map(|(id, o)| o.features().block_issuer().map(|f| (id, f)))
        {
            if let Some(input_feature) = account_inputs
                .get(&account_id)
                .and_then(|o| o.features().block_issuer())
            {
                if input_feature.block_issuer_keys() != output_feature.block_issuer_keys() {
                    self.block_issuer_key_rotated += 1;
                }
            }
        }
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
            ledger_output.output().as_anchor_opt().map(|output| AnchorData {
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
        let map = |ledger_output: &LedgerOutput| ledger_output.output().as_foundry_opt().map(|output| output.id());

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
    pub(crate) delayed_count: usize,
    pub(crate) destroyed_count: usize,
}

impl DelegationActivityMeasurement {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let map = |ledger_output: &LedgerOutput| {
            ledger_output
                .output()
                .as_delegation_opt()
                .map(|output| output.delegation_id_non_null(&ledger_output.output_id))
        };
        let delegation_inputs = consumed
            .iter()
            .map(|o| &o.output)
            .filter_map(map)
            .collect::<HashSet<_>>();

        let delegation_outputs = created.iter().filter_map(map).collect::<HashSet<_>>();

        self.created_count += delegation_outputs.difference(&delegation_inputs).count();
        // self.delayed_count += todo!();
        self.destroyed_count += delegation_inputs.difference(&delegation_outputs).count();
    }
}

/// Delegation activity statistics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct NativeTokenActivityMeasurement {
    pub(crate) minted_count: usize,
    pub(crate) melted_count: usize,
}

impl NativeTokenActivityMeasurement {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput]) {
        let map = |ledger_output: &LedgerOutput| ledger_output.output().native_token().map(|nt| *nt.token_id());
        let native_token_inputs = consumed
            .iter()
            .map(|o| &o.output)
            .filter_map(map)
            .collect::<HashSet<_>>();

        let native_token_outputs = created.iter().filter_map(map).collect::<HashSet<_>>();

        self.minted_count += native_token_outputs.difference(&native_token_inputs).count();
        self.melted_count += native_token_inputs.difference(&native_token_outputs).count();
    }
}

trait SetOps {
    fn difference_count(&self, other: &Self) -> usize;

    fn intersection_count(&self, other: &Self) -> usize;
}

impl<K: Eq + core::hash::Hash> SetOps for HashSet<K> {
    fn difference_count(&self, other: &Self) -> usize {
        self.difference(other).count()
    }

    fn intersection_count(&self, other: &Self) -> usize {
        self.intersection(other).count()
    }
}

impl<K: Eq + core::hash::Hash, V> SetOps for HashMap<K, V> {
    fn difference_count(&self, other: &Self) -> usize {
        self.keys().filter(|k| !other.contains_key(k)).count()
    }

    fn intersection_count(&self, other: &Self) -> usize {
        self.keys().filter(|k| other.contains_key(k)).count()
    }
}
