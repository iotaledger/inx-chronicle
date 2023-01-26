// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::types::{
    ledger::{LedgerOutput, LedgerSpent},
    stardust::block::output::OutputId,
};

/// Holds the ledger updates that happened during a milestone.
///
/// Note: For now we store all of these in memory. At some point we might need to retrieve them from an async
/// datasource.
#[derive(Clone, Default)]
#[allow(missing_docs)]
pub struct LedgerUpdateStore {
    pub created: HashMap<OutputId, LedgerOutput>,
    pub consumed: HashMap<OutputId, LedgerSpent>,
}

impl LedgerUpdateStore {
    /// Retrieves a [`LedgerOutput`] by [`OutputId`].
    ///
    /// Note: Only outputs that were touched in the current milestone (either as inputs or outputs) are present.
    pub fn get_created(&self, output_id: &OutputId) -> Option<&LedgerOutput> {
        self.created.get(output_id)
    }

    /// Retrieves a [`LedgerSpent`] by [`OutputId`].
    ///
    /// Note: Only outputs that were touched in the current milestone (either as inputs or outputs) are present.
    pub fn get_consumed(&self, output_id: &OutputId) -> Option<&LedgerSpent> {
        self.consumed.get(output_id)
    }
}
