// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::types::stardust::block::{output::OutputId, Output};

/// Holds the ledger updates that happened during a milestone.
///
/// Note: For now we store all of these in memory. At some point we might need to retrieve them from an async
/// datasource.
pub struct LedgerUpdateStore {
    outputs: HashMap<OutputId, Output>,
}

impl LedgerUpdateStore {
    /// Retrieves an [`Output`] by [`OutputId`].
    ///
    /// Note: Only outputs that were touched in the current milestone (either as inputs or outputs) are present.
    pub fn get_output(&self, output_id: &OutputId) -> Option<&Output> {
        self.outputs.get(&output_id)
    }
}
