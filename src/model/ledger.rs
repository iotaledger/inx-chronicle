// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains ledger types.

use std::collections::HashMap;

use iota_sdk::types::block::{
    address::Address,
    output::{Output, OutputId},
    payload::signed_transaction::TransactionId,
    protocol::ProtocolParameters,
    slot::{SlotCommitmentId, SlotIndex},
    BlockId,
};
use serde::{Deserialize, Serialize};

use super::raw::Raw;

/// An unspent output according to the ledger.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerOutput {
    pub output_id: OutputId,
    pub block_id: BlockId,
    pub slot_booked: SlotIndex,
    pub commitment_id_included: SlotCommitmentId,
    pub output: Raw<Output>,
}

#[allow(missing_docs)]
impl LedgerOutput {
    pub fn output_id(&self) -> OutputId {
        self.output_id
    }

    pub fn output(&self) -> &Output {
        self.output.inner()
    }

    pub fn amount(&self) -> u64 {
        self.output().amount()
    }

    pub fn mana(&self) -> u64 {
        self.output().mana()
    }

    pub fn owning_address(&self) -> Address {
        match self.output() {
            Output::Basic(output) => output.address().clone(),
            Output::Account(output) => output.address().clone(),
            Output::Anchor(output) => output.state_controller_address().clone(),
            Output::Foundry(output) => Address::from(*output.account_address()),
            Output::Nft(output) => output.address().clone(),
            Output::Delegation(output) => output.address().clone(),
        }
    }

    /// Returns the [`Address`] that is in control of the output at the given slot.
    pub fn locked_address_at(&self, slot: impl Into<SlotIndex>, protocol_parameters: &ProtocolParameters) -> Address {
        let owning_address = self.owning_address();
        self.output()
            .unlock_conditions()
            .unwrap()
            .locked_address(
                &owning_address,
                slot.into(),
                protocol_parameters.committable_age_range(),
            )
            .unwrap()
            .cloned()
            .unwrap_or(owning_address)
    }

    /// Returns the [`Address`] that is in control of the output at the booked slot.
    pub fn locked_address(&self, protocol_parameters: &ProtocolParameters) -> Address {
        self.locked_address_at(self.slot_booked, protocol_parameters)
    }

    pub fn kind(&self) -> &str {
        match self.output() {
            Output::Basic(_) => "basic",
            Output::Account(_) => "account",
            Output::Anchor(_) => "anchor",
            Output::Foundry(_) => "foundry",
            Output::Nft(_) => "nft",
            Output::Delegation(_) => "delegation",
        }
    }
}

/// A spent output according to the ledger.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerSpent {
    pub output: LedgerOutput,
    pub commitment_id_spent: SlotCommitmentId,
    pub transaction_id_spent: TransactionId,
    pub slot_spent: SlotIndex,
}

#[allow(missing_docs)]
impl LedgerSpent {
    pub fn output_id(&self) -> OutputId {
        self.output.output_id
    }

    pub fn output(&self) -> &Output {
        self.output.output()
    }

    pub fn amount(&self) -> u64 {
        self.output().amount()
    }

    pub fn slot_booked(&self) -> SlotIndex {
        self.output.slot_booked
    }

    pub fn owning_address(&self) -> Address {
        self.output.owning_address()
    }

    /// Returns the [`Address`] that is in control of the output at the given slot.
    pub fn locked_address_at(&self, slot: impl Into<SlotIndex>, protocol_parameters: &ProtocolParameters) -> Address {
        self.output.locked_address_at(slot, protocol_parameters)
    }

    /// Returns the [`Address`] that is in control of the output at the spent slot.
    pub fn locked_address(&self, protocol_parameters: &ProtocolParameters) -> Address {
        self.locked_address_at(self.slot_spent, protocol_parameters)
    }
}

/// Holds the ledger updates that happened during a slot.
///
/// Note: For now we store all of these in memory. At some point we might need to retrieve them from an async
/// datasource.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerUpdateStore {
    created: Vec<LedgerOutput>,
    created_index: HashMap<OutputId, usize>,
    consumed: Vec<LedgerSpent>,
    consumed_index: HashMap<OutputId, usize>,
}

impl LedgerUpdateStore {
    /// Initializes the store with consumed and created outputs.
    pub fn init(consumed: Vec<LedgerSpent>, created: Vec<LedgerOutput>) -> Self {
        let mut consumed_index = HashMap::new();
        for (idx, c) in consumed.iter().enumerate() {
            consumed_index.insert(c.output_id(), idx);
        }

        let mut created_index = HashMap::new();
        for (idx, c) in created.iter().enumerate() {
            created_index.insert(c.output_id(), idx);
        }

        LedgerUpdateStore {
            created,
            created_index,
            consumed,
            consumed_index,
        }
    }

    /// Retrieves a [`LedgerOutput`] by [`OutputId`].
    ///
    /// Note: Only outputs that were touched in the current slot (either as inputs or outputs) are present.
    pub fn get_created(&self, output_id: &OutputId) -> Option<&LedgerOutput> {
        self.created_index.get(output_id).map(|&idx| &self.created[idx])
    }

    /// Retrieves a [`LedgerSpent`] by [`OutputId`].
    ///
    /// Note: Only outputs that were touched in the current slot (either as inputs or outputs) are present.
    pub fn get_consumed(&self, output_id: &OutputId) -> Option<&LedgerSpent> {
        self.consumed_index.get(output_id).map(|&idx| &self.consumed[idx])
    }

    /// The list of spent outputs.
    pub fn consumed_outputs(&self) -> &[LedgerSpent] {
        &self.consumed
    }

    /// The list of created outputs.
    pub fn created_outputs(&self) -> &[LedgerOutput] {
        &self.created
    }
}
