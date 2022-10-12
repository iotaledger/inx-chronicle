// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::{ConflictReason, LedgerInclusionState};
use crate::types::{stardust::block::BlockId, tangle::MilestoneIndex};

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

#[cfg(feature = "inx")]
impl TryFrom<inx::proto::BlockMetadata> for BlockMetadata {
    type Error = crate::inx::InxError;

    fn try_from(value: inx::proto::BlockMetadata) -> Result<Self, Self::Error> {
        let inclusion_state = value.ledger_inclusion_state().into();
        let conflict_reason = value.conflict_reason().into();

        let parents = value
            .parents
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(BlockMetadata {
            parents: parents.into_boxed_slice(),
            is_solid: value.solid,
            should_promote: value.should_promote,
            should_reattach: value.should_reattach,
            referenced_by_milestone_index: value.referenced_by_milestone_index.into(),
            milestone_index: value.milestone_index.into(),
            inclusion_state,
            conflict_reason,
            white_flag_index: value.white_flag_index,
        })
    }
}
