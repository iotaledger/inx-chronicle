// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use inx::proto;
use iota_sdk::types::block::{
    payload::signed_transaction::TransactionId,
    slot::{SlotCommitmentId, SlotIndex},
};

use super::{
    convert::{ConvertFrom, TryConvertFrom, TryConvertTo},
    InxError,
};
use crate::{
    maybe_missing,
    model::{
        block_metadata::{BlockFailureReason, BlockState, TransactionFailureReason, TransactionState},
        ledger::{LedgerOutput, LedgerSpent},
    },
};

impl TryConvertFrom<proto::LedgerOutput> for LedgerOutput {
    type Error = InxError;

    fn try_convert_from(proto: proto::LedgerOutput) -> Result<Self, Self::Error> {
        Ok(Self {
            output_id: maybe_missing!(proto.output_id).try_convert()?,
            block_id: maybe_missing!(proto.block_id).try_convert()?,
            slot_booked: proto.slot_booked.into(),
            commitment_id_included: maybe_missing!(proto.commitment_id_included).try_convert()?,
            output: maybe_missing!(proto.output).try_into()?,
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

    /// If present, returns the `Marker` that denotes the beginning of a slot while consuming `self`.
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

impl TryConvertFrom<inx::proto::ledger_update::Marker> for MarkerMessage {
    type Error = InxError;

    fn try_convert_from(value: inx::proto::ledger_update::Marker) -> Result<Self, Self::Error> {
        Ok(Self {
            slot_index: SlotCommitmentId::try_convert_from(maybe_missing!(value.commitment_id))?.slot_index(),
            consumed_count: value.consumed_count as usize,
            created_count: value.created_count as usize,
        })
    }
}

impl TryConvertFrom<inx::proto::ledger_update::Marker> for LedgerUpdate {
    type Error = InxError;

    fn try_convert_from(value: inx::proto::ledger_update::Marker) -> Result<Self, Self::Error> {
        use inx::proto::ledger_update::marker::MarkerType as proto;
        Ok(match value.marker_type() {
            proto::Begin => Self::Begin(value.try_convert()?),
            proto::End => Self::End(value.try_convert()?),
        })
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
            proto::BatchMarker(marker) => marker.try_convert()?,
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
            ProtoState::Accepted => BlockState::Accepted,
            ProtoState::Unknown => BlockState::Unknown,
        }
    }
}

impl ConvertFrom<proto::transaction_metadata::TransactionState> for TransactionState {
    fn convert_from(proto: proto::transaction_metadata::TransactionState) -> Self {
        use proto::transaction_metadata::TransactionState as ProtoState;
        match proto {
            ProtoState::Pending => TransactionState::Pending,
            ProtoState::Confirmed => TransactionState::Confirmed,
            ProtoState::Finalized => TransactionState::Finalized,
            ProtoState::Failed => TransactionState::Failed,
            ProtoState::Accepted => TransactionState::Accepted,
            ProtoState::NoTransaction => panic!("tried to convert a transaction state where no transaction exists"),
        }
    }
}

impl ConvertFrom<proto::block_metadata::BlockFailureReason> for Option<BlockFailureReason> {
    fn convert_from(proto: proto::block_metadata::BlockFailureReason) -> Self {
        use proto::block_metadata::BlockFailureReason as ProtoState;
        Some(match proto {
            ProtoState::None => return None,
            ProtoState::IsTooOld => BlockFailureReason::TooOldToIssue,
            ProtoState::ParentIsTooOld => BlockFailureReason::ParentTooOld,
            ProtoState::ParentNotFound => BlockFailureReason::ParentDoesNotExist,
            ProtoState::ParentInvalid => BlockFailureReason::ParentInvalid,
            ProtoState::IssuerAccountNotFound => BlockFailureReason::IssuerAccountNotFound,
            ProtoState::VersionInvalid => BlockFailureReason::VersionInvalid,
            ProtoState::ManaCostCalculationFailed => BlockFailureReason::ManaCostCalculationFailed,
            ProtoState::BurnedInsufficientMana => BlockFailureReason::BurnedInsufficientMana,
            ProtoState::AccountInvalid => BlockFailureReason::AccountInvalid,
            ProtoState::SignatureInvalid => BlockFailureReason::SignatureInvalid,
            ProtoState::DroppedDueToCongestion => BlockFailureReason::DroppedDueToCongestion,
            ProtoState::PayloadInvalid => BlockFailureReason::PayloadInvalid,
            ProtoState::FailureInvalid => BlockFailureReason::Invalid,
        })
    }
}

impl ConvertFrom<proto::transaction_metadata::TransactionFailureReason> for Option<TransactionFailureReason> {
    fn convert_from(proto: proto::transaction_metadata::TransactionFailureReason) -> Self {
        use proto::transaction_metadata::TransactionFailureReason as ProtoState;
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
