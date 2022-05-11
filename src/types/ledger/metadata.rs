// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::{ConflictReason, LedgerInclusionState};

/// Message metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    /// Status of the solidification process.
    pub is_solid: bool,
    /// Indicates that the message should be promoted.
    pub should_promote: bool,
    /// Indicates that the message should be reattached.
    pub should_reattach: bool,
    /// The milestone index referencing the message.
    pub referenced_by_milestone_index: u32,
    /// The corresponding milestone index.
    pub milestone_index: u32,
    /// The inclusion state of the message.
    pub inclusion_state: LedgerInclusionState,
    /// If the ledger inclusion state is conflicting, the reason for the conflict.
    pub conflict_reason: Option<ConflictReason>,
}
