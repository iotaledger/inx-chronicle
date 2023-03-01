// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::{ConflictReason, LedgerInclusionState};
use crate::model::{payload::milestone::MilestoneIndex, BlockId};

/// Block metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMetadata {
    /// The parents of the corresponding block.
    pub parents: Box<[BlockId]>,
    /// Status of the solidification process.
    pub is_solid: bool,
    /// Indicates that the block should be promoted.
    pub should_promote: bool,
    /// Indicates that the block should be reattached.
    pub should_reattach: bool,
    /// The milestone index referencing the block.
    pub referenced_by_milestone_index: MilestoneIndex,
    /// The corresponding milestone index.
    pub milestone_index: MilestoneIndex,
    /// The inclusion state of the block.
    pub inclusion_state: LedgerInclusionState,
    /// If the ledger inclusion state is conflicting, the reason for the conflict.
    pub conflict_reason: ConflictReason,
    /// The index of this block in white flag order.
    pub white_flag_index: u32,
}
