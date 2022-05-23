// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::{ConflictReason, LedgerInclusionState};

/// Block metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    /// Status of the solidification process.
    pub is_solid: bool,
    /// Indicates that the block should be promoted.
    pub should_promote: bool,
    /// Indicates that the block should be reattached.
    pub should_reattach: bool,
    /// The milestone index referencing the block.
    pub referenced_by_milestone_index: u32,
    /// The corresponding milestone index.
    pub milestone_index: u32,
    /// The inclusion state of the block.
    pub inclusion_state: LedgerInclusionState,
    /// If the ledger inclusion state is conflicting, the reason for the conflict.
    pub conflict_reason: ConflictReason,
}

#[cfg(feature = "inx")]
impl From<inx::BlockMetadata> for Metadata {
    fn from(metadata: inx::BlockMetadata) -> Self {
        Self {
            is_solid: metadata.is_solid,
            should_promote: metadata.should_promote,
            should_reattach: metadata.should_reattach,
            referenced_by_milestone_index: metadata.referenced_by_milestone_index,
            milestone_index: metadata.milestone_index,
            inclusion_state: metadata.ledger_inclusion_state.into(),
            conflict_reason: metadata.conflict_reason.into(),
        }
    }
}
