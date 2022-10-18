// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust as bee;
use packable::PackableExt;

use super::InxError;
use crate::{
    maybe_missing,
    types::{
        context::{TryFromWithContext, TryIntoWithContext},
        ledger::{ConflictReason, LedgerInclusionState, LedgerOutput, LedgerSpent},
        tangle::MilestoneIndex,
    },
};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnspentOutputMessage {
    pub ledger_index: MilestoneIndex,
    pub output: LedgerOutput,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerMessage {
    pub milestone_index: MilestoneIndex,
    pub consumed_count: usize,
    pub created_count: usize,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerUpdateMessage {
    Consumed(LedgerSpent),
    Created(LedgerOutput),
    Begin(MarkerMessage),
    End(MarkerMessage),
}

impl LedgerUpdateMessage {
    /// If present, returns the contained `LedgerSpent` while consuming `self`.
    pub fn consumed(self) -> Option<LedgerSpent> {
        match self {
            Self::Consumed(ledger_spent) => Some(ledger_spent),
            _ => None,
        }
    }

    /// If present, returns the contained `LedgerOutput` while consuming `self`.
    pub fn created(self) -> Option<LedgerOutput> {
        match self {
            Self::Created(ledger_output) => Some(ledger_output),
            _ => None,
        }
    }

    /// If present, returns the `Marker` that denotes the beginning of a milestone while consuming `self`.
    pub fn begin(self) -> Option<MarkerMessage> {
        match self {
            Self::Begin(marker) => Some(marker),
            _ => None,
        }
    }

    /// If present, returns the `Marker` that denotes the end if present while consuming `self`.
    pub fn end(self) -> Option<MarkerMessage> {
        match self {
            Self::End(marker) => Some(marker),
            _ => None,
        }
    }
}

impl From<inx::proto::ledger_update::Marker> for MarkerMessage {
    fn from(value: inx::proto::ledger_update::Marker) -> Self {
        Self {
            milestone_index: value.milestone_index.into(),
            consumed_count: value.consumed_count as usize,
            created_count: value.created_count as usize,
        }
    }
}

impl From<inx::proto::ledger_update::Marker> for LedgerUpdateMessage {
    fn from(value: inx::proto::ledger_update::Marker) -> Self {
        use inx::proto::ledger_update::marker::MarkerType as proto;
        match value.marker_type() {
            proto::Begin => Self::Begin(value.into()),
            proto::End => Self::End(value.into()),
        }
    }
}

impl TryFrom<inx::proto::LedgerUpdate> for LedgerUpdateMessage {
    type Error = InxError;

    fn try_from(value: inx::proto::LedgerUpdate) -> Result<Self, Self::Error> {
        use inx::proto::ledger_update::Op as proto;
        Ok(match maybe_missing!(value.op) {
            proto::BatchMarker(marker) => marker.into(),
            proto::Consumed(consumed) => LedgerUpdateMessage::Consumed(consumed.try_into()?),
            proto::Created(created) => LedgerUpdateMessage::Created(created.try_into()?),
        })
    }
}

impl TryFrom<inx::proto::UnspentOutput> for UnspentOutputMessage {
    type Error = InxError;

    fn try_from(value: inx::proto::UnspentOutput) -> Result<Self, Self::Error> {
        Ok(Self {
            ledger_index: value.ledger_index.into(),
            output: maybe_missing!(value.output).try_into()?,
        })
    }
}

impl TryFromWithContext<UnspentOutputMessage> for inx::proto::UnspentOutput {
    type Error = bee::Error;

    fn try_from_with_context(
        ctx: &bee::protocol::ProtocolParameters,
        value: UnspentOutputMessage,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            ledger_index: value.ledger_index.0,
            output: Some(value.output.try_into_with_context(ctx)?),
        })
    }
}

impl TryFromWithContext<LedgerOutput> for inx::proto::LedgerOutput {
    type Error = bee::Error;

    fn try_from_with_context(
        ctx: &bee::protocol::ProtocolParameters,
        value: LedgerOutput,
    ) -> Result<Self, Self::Error> {
        let bee_output = bee::output::Output::try_from_with_context(ctx, value.output)?;

        Ok(Self {
            block_id: Some(value.block_id.into()),
            milestone_index_booked: value.booked.milestone_index.0,
            milestone_timestamp_booked: value.booked.milestone_timestamp.0,
            output: Some(inx::proto::RawOutput {
                data: bee_output.pack_to_vec(),
            }),
            output_id: Some(value.output_id.into()),
        })
    }
}

impl From<inx::proto::block_metadata::LedgerInclusionState> for LedgerInclusionState {
    fn from(value: inx::proto::block_metadata::LedgerInclusionState) -> Self {
        use inx::proto::block_metadata::LedgerInclusionState;
        match value {
            LedgerInclusionState::Included => Self::Included,
            LedgerInclusionState::NoTransaction => Self::NoTransaction,
            LedgerInclusionState::Conflicting => Self::Conflicting,
        }
    }
}

impl From<LedgerInclusionState> for inx::proto::block_metadata::LedgerInclusionState {
    fn from(value: LedgerInclusionState) -> Self {
        match value {
            LedgerInclusionState::Included => Self::Included,
            LedgerInclusionState::NoTransaction => Self::NoTransaction,
            LedgerInclusionState::Conflicting => Self::Conflicting,
        }
    }
}

impl From<inx::proto::block_metadata::ConflictReason> for ConflictReason {
    fn from(value: inx::proto::block_metadata::ConflictReason) -> Self {
        use ::inx::proto::block_metadata::ConflictReason;
        match value {
            ConflictReason::None => Self::None,
            ConflictReason::InputAlreadySpent => Self::InputUtxoAlreadySpent,
            ConflictReason::InputAlreadySpentInThisMilestone => Self::InputUtxoAlreadySpentInThisMilestone,
            ConflictReason::InputNotFound => Self::InputUtxoNotFound,
            ConflictReason::InputOutputSumMismatch => Self::CreatedConsumedAmountMismatch,
            ConflictReason::InvalidSignature => Self::InvalidSignature,
            ConflictReason::TimelockNotExpired => Self::TimelockNotExpired,
            ConflictReason::InvalidNativeTokens => Self::InvalidNativeTokens,
            ConflictReason::ReturnAmountNotFulfilled => Self::StorageDepositReturnUnfulfilled,
            ConflictReason::InvalidInputUnlock => Self::InvalidUnlock,
            ConflictReason::InvalidInputsCommitment => Self::InputsCommitmentsMismatch,
            ConflictReason::InvalidSender => Self::UnverifiedSender,
            ConflictReason::InvalidChainStateTransition => Self::InvalidChainStateTransition,
            ConflictReason::SemanticValidationFailed => Self::SemanticValidationFailed,
        }
    }
}

impl From<ConflictReason> for inx::proto::block_metadata::ConflictReason {
    fn from(value: ConflictReason) -> Self {
        match value {
            ConflictReason::None => Self::None,
            ConflictReason::InputUtxoAlreadySpent => Self::InputAlreadySpent,
            ConflictReason::InputUtxoAlreadySpentInThisMilestone => Self::InputAlreadySpentInThisMilestone,
            ConflictReason::InputUtxoNotFound => Self::InputNotFound,
            ConflictReason::CreatedConsumedAmountMismatch => Self::InputOutputSumMismatch,
            ConflictReason::InvalidSignature => Self::InvalidSignature,
            ConflictReason::TimelockNotExpired => Self::TimelockNotExpired,
            ConflictReason::InvalidNativeTokens => Self::InvalidNativeTokens,
            ConflictReason::StorageDepositReturnUnfulfilled => Self::ReturnAmountNotFulfilled,
            ConflictReason::InvalidUnlock => Self::InvalidInputUnlock,
            ConflictReason::InputsCommitmentsMismatch => Self::InvalidInputsCommitment,
            ConflictReason::UnverifiedSender => Self::InvalidSender,
            ConflictReason::InvalidChainStateTransition => Self::InvalidChainStateTransition,
            ConflictReason::SemanticValidationFailed => Self::SemanticValidationFailed,
        }
    }
}
