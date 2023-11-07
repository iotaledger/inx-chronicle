// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use inx::proto;
use iota_sdk::types::{
    api::core::{BlockFailureReason, BlockState, TransactionState},
    block::{
        address::Address,
        output::{Output, OutputId},
        payload::signed_transaction::TransactionId,
        semantic::TransactionFailureReason,
        slot::{SlotCommitmentId, SlotIndex},
        BlockId,
    },
};

use super::{
    convert::{ConvertFrom, TryConvertFrom, TryConvertTo},
    InxError,
};
use crate::maybe_missing;

/// An unspent output according to the ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub struct LedgerOutput {
    pub output_id: OutputId,
    pub block_id: BlockId,
    pub slot_booked: SlotIndex,
    pub commitment_id_included: SlotCommitmentId,
    pub output: Output,
}

#[allow(missing_docs)]
impl LedgerOutput {
    pub fn output_id(&self) -> OutputId {
        self.output_id
    }

    pub fn output(&self) -> &Output {
        &self.output
    }

    pub fn amount(&self) -> u64 {
        self.output().amount()
    }

    pub fn address(&self) -> Option<&Address> {
        self.output()
            .unlock_conditions()
            .and_then(|uc| uc.address())
            .map(|uc| uc.address())
    }
}

/// A spent output according to the ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub struct LedgerSpent {
    pub output: LedgerOutput,
    pub commitment_id_spent: SlotCommitmentId,
    pub transaction_id_spent: TransactionId,
    pub slot_spent: SlotIndex,
}

#[allow(missing_docs)]
impl LedgerSpent {
    pub fn output_id(&self) -> OutputId {
        self.output.output_id
    }

    pub fn output(&self) -> &Output {
        &self.output.output()
    }

    pub fn amount(&self) -> u64 {
        self.output().amount()
    }

    pub fn address(&self) -> Option<&Address> {
        self.output.address()
    }
}

impl TryConvertFrom<proto::LedgerOutput> for LedgerOutput {
    type Error = InxError;

    fn try_convert_from(proto: proto::LedgerOutput) -> Result<Self, Self::Error> {
        Ok(Self {
            output_id: maybe_missing!(proto.output_id).try_convert()?,
            block_id: maybe_missing!(proto.block_id).try_convert()?,
            slot_booked: proto.slot_booked.into(),
            commitment_id_included: maybe_missing!(proto.commitment_id_included).try_convert()?,
            output: maybe_missing!(proto.output).try_convert()?,
        })
    }
}

impl TryConvertFrom<proto::LedgerSpent> for LedgerSpent {
    type Error = InxError;

    fn try_convert_from(proto: proto::LedgerSpent) -> Result<Self, Self::Error> {
        Ok(Self {
            output: maybe_missing!(proto.output).try_convert()?,
            commitment_id_spent: maybe_missing!(proto.commitment_id_spent).try_convert()?,
            transaction_id_spent: maybe_missing!(proto.transaction_id_spent).try_convert()?,
            slot_spent: proto.slot_spent.into(),
        })
    }
}

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

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnspentOutput {
    pub latest_commitment_id: SlotCommitmentId,
    pub output: LedgerOutput,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerMessage {
    pub slot_index: SlotIndex,
    pub consumed_count: usize,
    pub created_count: usize,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerUpdate {
    Consumed(LedgerSpent),
    Created(LedgerOutput),
    Begin(MarkerMessage),
    End(MarkerMessage),
}

impl LedgerUpdate {
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
            slot_index: value.slot.into(),
            consumed_count: value.consumed_count as usize,
            created_count: value.created_count as usize,
        }
    }
}

impl From<inx::proto::ledger_update::Marker> for LedgerUpdate {
    fn from(value: inx::proto::ledger_update::Marker) -> Self {
        use inx::proto::ledger_update::marker::MarkerType as proto;
        match value.marker_type() {
            proto::Begin => Self::Begin(value.into()),
            proto::End => Self::End(value.into()),
        }
    }
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedTransaction {
    pub transaction_id: TransactionId,
    pub slot_index: SlotIndex,
    pub consumed: Vec<LedgerSpent>,
    pub created: Vec<LedgerOutput>,
}

impl TryConvertFrom<inx::proto::LedgerUpdate> for LedgerUpdate {
    type Error = InxError;

    fn try_convert_from(proto: inx::proto::LedgerUpdate) -> Result<Self, Self::Error> {
        use inx::proto::ledger_update::Op as proto;
        Ok(match maybe_missing!(proto.op) {
            proto::BatchMarker(marker) => marker.into(),
            proto::Consumed(consumed) => LedgerUpdate::Consumed(consumed.try_convert()?),
            proto::Created(created) => LedgerUpdate::Created(created.try_convert()?),
        })
    }
}

impl TryConvertFrom<inx::proto::UnspentOutput> for UnspentOutput {
    type Error = InxError;

    fn try_convert_from(proto: inx::proto::UnspentOutput) -> Result<Self, Self::Error> {
        Ok(Self {
            latest_commitment_id: maybe_missing!(proto.latest_commitment_id).try_convert()?,
            output: maybe_missing!(proto.output).try_convert()?,
        })
    }
}

impl TryConvertFrom<inx::proto::AcceptedTransaction> for AcceptedTransaction {
    type Error = InxError;

    fn try_convert_from(proto: inx::proto::AcceptedTransaction) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_id: maybe_missing!(proto.transaction_id).try_convert()?,
            slot_index: proto.slot.into(),
            consumed: proto
                .consumed
                .into_iter()
                .map(TryConvertTo::try_convert)
                .collect::<Result<_, _>>()?,
            created: proto
                .created
                .into_iter()
                .map(TryConvertTo::try_convert)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl ConvertFrom<proto::block_metadata::BlockState> for BlockState {
    fn convert_from(proto: proto::block_metadata::BlockState) -> Self {
        use proto::block_metadata::BlockState as ProtoState;
        match proto {
            ProtoState::Pending => BlockState::Pending,
            ProtoState::Confirmed => BlockState::Confirmed,
            ProtoState::Finalized => BlockState::Finalized,
            ProtoState::Rejected => BlockState::Rejected,
            ProtoState::Failed => BlockState::Failed,
            ProtoState::Accepted => todo!(),
            ProtoState::Unknown => todo!(),
        }
    }
}

impl ConvertFrom<proto::block_metadata::TransactionState> for Option<TransactionState> {
    fn convert_from(proto: proto::block_metadata::TransactionState) -> Self {
        use proto::block_metadata::TransactionState as ProtoState;
        Some(match proto {
            ProtoState::NoTransaction => return None,
            ProtoState::Pending => TransactionState::Pending,
            ProtoState::Confirmed => TransactionState::Confirmed,
            ProtoState::Finalized => TransactionState::Finalized,
            ProtoState::Failed => TransactionState::Failed,
            ProtoState::Accepted => todo!(),
        })
    }
}

impl ConvertFrom<proto::block_metadata::BlockFailureReason> for Option<BlockFailureReason> {
    fn convert_from(proto: proto::block_metadata::BlockFailureReason) -> Self {
        use proto::block_metadata::BlockFailureReason as ProtoState;
        Some(match proto {
            ProtoState::None => return None,
            ProtoState::IsTooOld => BlockFailureReason::TooOldToIssue,
            ProtoState::ParentIsTooOld => BlockFailureReason::ParentTooOld,
            ProtoState::BookingFailure => todo!(),
            ProtoState::DroppedDueToCongestion => BlockFailureReason::DroppedDueToCongestion,
            ProtoState::PayloadInvalid => BlockFailureReason::PayloadInvalid,
            ProtoState::OrphanedDueNegativeCreditsBalance => todo!(),
        })
    }
}

impl ConvertFrom<proto::block_metadata::TransactionFailureReason> for Option<TransactionFailureReason> {
    fn convert_from(proto: proto::block_metadata::TransactionFailureReason) -> Self {
        use proto::block_metadata::TransactionFailureReason as ProtoState;
        Some(match proto {
            ProtoState::None => return None,
            ProtoState::UtxoInputAlreadySpent => TransactionFailureReason::InputUtxoAlreadySpent,
            ProtoState::Conflicting => TransactionFailureReason::ConflictingWithAnotherTx,
            ProtoState::UtxoInputInvalid => TransactionFailureReason::InvalidReferencedUtxo,
            ProtoState::TxTypeInvalid => TransactionFailureReason::InvalidTransaction,
            ProtoState::SumOfInputAndOutputValuesDoesNotMatch => {
                TransactionFailureReason::SumInputsOutputsAmountMismatch
            }
            ProtoState::UnlockBlockSignatureInvalid => TransactionFailureReason::InvalidUnlockBlockSignature,
            ProtoState::ConfiguredTimelockNotYetExpired => TransactionFailureReason::TimelockNotExpired,
            ProtoState::GivenNativeTokensInvalid => TransactionFailureReason::InvalidNativeTokens,
            ProtoState::ReturnAmountNotFulfilled => TransactionFailureReason::StorageDepositReturnUnfulfilled,
            ProtoState::InputUnlockInvalid => TransactionFailureReason::InvalidInputUnlock,
            ProtoState::SenderNotUnlocked => TransactionFailureReason::SenderNotUnlocked,
            ProtoState::ChainStateTransitionInvalid => TransactionFailureReason::InvalidChainStateTransition,
            ProtoState::InputCreationAfterTxCreation => TransactionFailureReason::InvalidTransactionIssuingTime,
            ProtoState::ManaAmountInvalid => TransactionFailureReason::InvalidManaAmount,
            ProtoState::BicInputInvalid => TransactionFailureReason::InvalidBlockIssuanceCreditsAmount,
            ProtoState::RewardInputInvalid => TransactionFailureReason::InvalidRewardContextInput,
            ProtoState::CommitmentInputInvalid => TransactionFailureReason::InvalidCommitmentContextInput,
            ProtoState::NoStakingFeature => TransactionFailureReason::MissingStakingFeature,
            ProtoState::FailedToClaimStakingReward => TransactionFailureReason::FailedToClaimStakingReward,
            ProtoState::FailedToClaimDelegationReward => TransactionFailureReason::FailedToClaimDelegationReward,
            ProtoState::CapabilitiesNativeTokenBurningNotAllowed => {
                TransactionFailureReason::TransactionCapabilityNativeTokenBurningNotAllowed
            }
            ProtoState::CapabilitiesManaBurningNotAllowed => {
                TransactionFailureReason::TransactionCapabilityManaBurningNotAllowed
            }
            ProtoState::CapabilitiesAccountDestructionNotAllowed => {
                TransactionFailureReason::TransactionCapabilityAccountDestructionNotAllowed
            }
            ProtoState::CapabilitiesAnchorDestructionNotAllowed => {
                TransactionFailureReason::TransactionCapabilityAnchorDestructionNotAllowed
            }
            ProtoState::CapabilitiesFoundryDestructionNotAllowed => {
                TransactionFailureReason::TransactionCapabilityFoundryDestructionNotAllowed
            }
            ProtoState::CapabilitiesNftDestructionNotAllowed => {
                TransactionFailureReason::TransactionCapabilityNftDestructionNotAllowed
            }
            ProtoState::SemanticValidationFailed => TransactionFailureReason::SemanticValidationFailed,
        })
    }
}
