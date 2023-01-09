use inx::proto;
use iota_types::block as iota;

use super::{InxError, RawMessage};
use crate::{
    maybe_missing,
    types::{
        ledger::{BlockMetadata, ConflictReason, LedgerInclusionState},
        stardust::block::BlockId,
        tangle::MilestoneIndex,
    },
};

/// The [`BlockMessage`] type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMessage {
    /// The [`BlockId`] of the block.
    pub block_id: BlockId,
    /// The complete [`Block`](iota::Block) as raw bytes.
    pub block: RawMessage<iota::Block>,
}

// Unfortunately, we can't reuse the `BlockMetadata` because we also require the `block_id`.
/// Block metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMetadataMessage {
    /// The id of the associated block.
    pub block_id: BlockId,
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

/// The [`BlockWithMetadataMessage`] type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockWithMetadataMessage {
    /// The [`BlockMetadataMessage`](BlockMetadataMessage) of the block.
    pub metadata: BlockMetadataMessage,
    /// The complete [`Block`](iota::Block) as raw bytes.
    pub block: RawMessage<iota::Block>,
}

impl TryFrom<inx::proto::BlockMetadata> for BlockMetadataMessage {
    type Error = crate::inx::InxError;

    fn try_from(value: inx::proto::BlockMetadata) -> Result<Self, Self::Error> {
        let inclusion_state = value.ledger_inclusion_state().into();
        let conflict_reason = value.conflict_reason().into();

        let parents = value
            .parents
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            block_id: maybe_missing!(value.block_id).try_into()?,
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

impl TryFrom<proto::BlockWithMetadata> for BlockWithMetadataMessage {
    type Error = InxError;

    fn try_from(value: proto::BlockWithMetadata) -> Result<Self, Self::Error> {
        Ok(BlockWithMetadataMessage {
            metadata: maybe_missing!(value.metadata).try_into()?,
            block: maybe_missing!(value.block).data.into(),
        })
    }
}

impl From<BlockMetadataMessage> for proto::BlockMetadata {
    fn from(value: BlockMetadataMessage) -> Self {
        Self {
            block_id: Some(value.block_id.into()),
            parents: value.parents.into_vec().into_iter().map(Into::into).collect(),
            solid: value.is_solid,
            should_promote: value.should_promote,
            should_reattach: value.should_reattach,
            referenced_by_milestone_index: value.referenced_by_milestone_index.0,
            milestone_index: value.milestone_index.0,
            ledger_inclusion_state: proto::block_metadata::LedgerInclusionState::from(value.inclusion_state).into(),
            conflict_reason: proto::block_metadata::ConflictReason::from(value.conflict_reason).into(),
            white_flag_index: value.white_flag_index,
        }
    }
}

impl From<BlockWithMetadataMessage> for proto::BlockWithMetadata {
    fn from(value: BlockWithMetadataMessage) -> Self {
        Self {
            metadata: Some(value.metadata.into()),
            block: Some(value.block.into()),
        }
    }
}

impl TryFrom<proto::Block> for BlockMessage {
    type Error = InxError;

    fn try_from(value: proto::Block) -> Result<Self, Self::Error> {
        Ok(BlockMessage {
            block_id: maybe_missing!(value.block_id).try_into()?,
            block: maybe_missing!(value.block).data.into(),
        })
    }
}

impl From<BlockMessage> for proto::Block {
    fn from(value: BlockMessage) -> Self {
        Self {
            block_id: Some(value.block_id.into()),
            block: Some(value.block.into()),
        }
    }
}

impl From<BlockMetadataMessage> for BlockMetadata {
    fn from(value: BlockMetadataMessage) -> Self {
        Self {
            parents: value.parents,
            is_solid: value.is_solid,
            should_reattach: value.should_reattach,
            should_promote: value.should_promote,
            milestone_index: value.milestone_index,
            referenced_by_milestone_index: value.referenced_by_milestone_index,
            inclusion_state: value.inclusion_state,
            conflict_reason: value.conflict_reason,
            white_flag_index: value.white_flag_index,
        }
    }
}
