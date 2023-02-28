// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::model::{
    ledger::{LedgerOutput, LedgerSpent},
    output::OutputId,
};

/// Holds the ledger updates that happened during a milestone.
///
/// Note: For now we store all of these in memory. At some point we might need to retrieve them from an async
/// datasource.
#[derive(Clone, Default)]
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
    /// Note: Only outputs that were touched in the current milestone (either as inputs or outputs) are present.
    pub fn get_created(&self, output_id: &OutputId) -> Option<&LedgerOutput> {
        self.created_index.get(output_id).map(|&idx| &self.created[idx])
    }

    /// Retrieves a [`LedgerSpent`] by [`OutputId`].
    ///
    /// Note: Only outputs that were touched in the current milestone (either as inputs or outputs) are present.
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
