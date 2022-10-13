// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust as bee;
use inx::proto;

use super::{InxError, RawMessage};
use crate::{
    maybe_missing,
    types::{ledger::BlockMetadata, stardust::block::BlockId},
};

/// The [`Block`] type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMessage {
    /// The [`BlockId`] of the block.
    pub block_id: BlockId,
    /// The complete [`Block`](bee::Block) as raw bytes.
    pub block: RawMessage<bee::Block>,
}

/// The [`BlockWithMetadata`] type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockWithMetadataMessage {
    /// The [`BlockMetadata`](BlockMetadata) of the block.
    pub metadata: BlockMetadata,
    /// The complete [`Block`](bee::Block) as raw bytes.
    pub block: RawMessage<bee::Block>,
}

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
            block_id: maybe_missing!(value.block_id),
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

impl From<BlockMetadata> for proto::BlockMetadata {
    fn from(value: BlockMetadata) -> Self {
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
