// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use inx::proto;
use iota_sdk::types::{
    api::core::{BlockState, TransactionState},
    block::{
        payload::signed_transaction::TransactionId,
        semantic::TransactionFailureReason,
        slot::{SlotCommitmentId, SlotIndex},
    },
};

use super::{
    convert::{ConvertFrom, TryConvertFrom, TryConvertTo},
    InxError,
};
use crate::{
    maybe_missing,
    model::ledger::{LedgerOutput, LedgerSpent},
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

impl ConvertFrom<proto::block_metadata::BlockState> for Option<BlockState> {
    fn convert_from(proto: proto::block_metadata::BlockState) -> Self {
        use proto::block_metadata::BlockState as ProtoState;
        Some(match proto {
            ProtoState::Pending => BlockState::Pending,
            ProtoState::Confirmed => BlockState::Confirmed,
            ProtoState::Finalized => BlockState::Finalized,
            ProtoState::Dropped => BlockState::Dropped,
            ProtoState::Orphaned => BlockState::Orphaned,
            ProtoState::Accepted => BlockState::Accepted,
            ProtoState::Unknown => return None,
        })
    }
}

impl ConvertFrom<proto::transaction_metadata::TransactionState> for Option<TransactionState> {
    fn convert_from(proto: proto::transaction_metadata::TransactionState) -> Self {
        use proto::transaction_metadata::TransactionState as ProtoState;
        Some(match proto {
            ProtoState::Pending => TransactionState::Pending,
            ProtoState::Committed => TransactionState::Committed,
            ProtoState::Finalized => TransactionState::Finalized,
            ProtoState::Failed => TransactionState::Failed,
            ProtoState::Accepted => TransactionState::Accepted,
            ProtoState::Unknown => return None,
        })
    }
}

impl ConvertFrom<proto::transaction_metadata::TransactionFailureReason> for Option<TransactionFailureReason> {
    fn convert_from(proto: proto::transaction_metadata::TransactionFailureReason) -> Self {
        use proto::transaction_metadata::TransactionFailureReason as ProtoState;
        Some(match proto {
            ProtoState::None => return None,
            ProtoState::ConflictRejected => TransactionFailureReason::ConflictRejected,
            ProtoState::Orphaned => TransactionFailureReason::Orphaned,
            ProtoState::InputAlreadySpent => TransactionFailureReason::InputAlreadySpent,
            ProtoState::InputCreationAfterTxCreation => TransactionFailureReason::InputCreationAfterTxCreation,
            ProtoState::UnlockSignatureInvalid => TransactionFailureReason::UnlockSignatureInvalid,
            ProtoState::ChainAddressUnlockInvalid => TransactionFailureReason::ChainAddressUnlockInvalid,
            ProtoState::DirectUnlockableAddressUnlockInvalid => {
                TransactionFailureReason::DirectUnlockableAddressUnlockInvalid
            }
            ProtoState::MultiAddressUnlockInvalid => TransactionFailureReason::MultiAddressUnlockInvalid,
            ProtoState::CommitmentInputReferenceInvalid => TransactionFailureReason::CommitmentInputReferenceInvalid,
            ProtoState::BicInputReferenceInvalid => TransactionFailureReason::BicInputReferenceInvalid,
            ProtoState::RewardInputReferenceInvalid => TransactionFailureReason::RewardInputReferenceInvalid,
            ProtoState::StakingRewardCalculationFailure => TransactionFailureReason::StakingRewardCalculationFailure,
            ProtoState::DelegationRewardCalculationFailure => {
                TransactionFailureReason::DelegationRewardCalculationFailure
            }
            ProtoState::InputOutputBaseTokenMismatch => TransactionFailureReason::InputOutputBaseTokenMismatch,
            ProtoState::ManaOverflow => TransactionFailureReason::ManaOverflow,
            ProtoState::InputOutputManaMismatch => TransactionFailureReason::InputOutputManaMismatch,
            ProtoState::ManaDecayCreationIndexExceedsTargetIndex => {
                TransactionFailureReason::ManaDecayCreationIndexExceedsTargetIndex
            }
            ProtoState::NativeTokenSumUnbalanced => TransactionFailureReason::NativeTokenSumUnbalanced,
            ProtoState::SimpleTokenSchemeMintedMeltedTokenDecrease => {
                TransactionFailureReason::SimpleTokenSchemeMintedMeltedTokenDecrease
            }
            ProtoState::SimpleTokenSchemeMintingInvalid => TransactionFailureReason::SimpleTokenSchemeMintingInvalid,
            ProtoState::SimpleTokenSchemeMeltingInvalid => TransactionFailureReason::SimpleTokenSchemeMeltingInvalid,
            ProtoState::SimpleTokenSchemeMaximumSupplyChanged => {
                TransactionFailureReason::SimpleTokenSchemeMaximumSupplyChanged
            }
            ProtoState::SimpleTokenSchemeGenesisInvalid => TransactionFailureReason::SimpleTokenSchemeGenesisInvalid,
            ProtoState::MultiAddressLengthUnlockLengthMismatch => {
                TransactionFailureReason::MultiAddressLengthUnlockLengthMismatch
            }
            ProtoState::MultiAddressUnlockThresholdNotReached => {
                TransactionFailureReason::MultiAddressUnlockThresholdNotReached
            }
            ProtoState::SenderFeatureNotUnlocked => TransactionFailureReason::SenderFeatureNotUnlocked,
            ProtoState::IssuerFeatureNotUnlocked => TransactionFailureReason::IssuerFeatureNotUnlocked,
            ProtoState::StakingRewardInputMissing => TransactionFailureReason::StakingRewardInputMissing,
            ProtoState::StakingCommitmentInputMissing => TransactionFailureReason::StakingCommitmentInputMissing,
            ProtoState::StakingRewardClaimingInvalid => TransactionFailureReason::StakingRewardClaimingInvalid,
            ProtoState::StakingFeatureRemovedBeforeUnbonding => {
                TransactionFailureReason::StakingFeatureRemovedBeforeUnbonding
            }
            ProtoState::StakingFeatureModifiedBeforeUnbonding => {
                TransactionFailureReason::StakingFeatureModifiedBeforeUnbonding
            }
            ProtoState::StakingStartEpochInvalid => TransactionFailureReason::StakingStartEpochInvalid,
            ProtoState::StakingEndEpochTooEarly => TransactionFailureReason::StakingEndEpochTooEarly,
            ProtoState::BlockIssuerCommitmentInputMissing => {
                TransactionFailureReason::BlockIssuerCommitmentInputMissing
            }
            ProtoState::BlockIssuanceCreditInputMissing => TransactionFailureReason::BlockIssuanceCreditInputMissing,
            ProtoState::BlockIssuerNotExpired => TransactionFailureReason::BlockIssuerNotExpired,
            ProtoState::BlockIssuerExpiryTooEarly => TransactionFailureReason::BlockIssuerExpiryTooEarly,
            ProtoState::ManaMovedOffBlockIssuerAccount => TransactionFailureReason::ManaMovedOffBlockIssuerAccount,
            ProtoState::AccountLocked => TransactionFailureReason::AccountLocked,
            ProtoState::TimelockCommitmentInputMissing => TransactionFailureReason::TimelockCommitmentInputMissing,
            ProtoState::TimelockNotExpired => TransactionFailureReason::TimelockNotExpired,
            ProtoState::ExpirationCommitmentInputMissing => TransactionFailureReason::ExpirationCommitmentInputMissing,
            ProtoState::ExpirationNotUnlockable => TransactionFailureReason::ExpirationNotUnlockable,
            ProtoState::ReturnAmountNotFulFilled => TransactionFailureReason::ReturnAmountNotFulFilled,
            ProtoState::NewChainOutputHasNonZeroedId => TransactionFailureReason::NewChainOutputHasNonZeroedId,
            ProtoState::ChainOutputImmutableFeaturesChanged => {
                TransactionFailureReason::ChainOutputImmutableFeaturesChanged
            }
            ProtoState::ImplicitAccountDestructionDisallowed => {
                TransactionFailureReason::ImplicitAccountDestructionDisallowed
            }
            ProtoState::MultipleImplicitAccountCreationAddresses => {
                TransactionFailureReason::MultipleImplicitAccountCreationAddresses
            }
            ProtoState::AccountInvalidFoundryCounter => TransactionFailureReason::AccountInvalidFoundryCounter,
            ProtoState::AnchorInvalidStateTransition => TransactionFailureReason::AnchorInvalidStateTransition,
            ProtoState::AnchorInvalidGovernanceTransition => {
                TransactionFailureReason::AnchorInvalidGovernanceTransition
            }
            ProtoState::FoundryTransitionWithoutAccount => TransactionFailureReason::FoundryTransitionWithoutAccount,
            ProtoState::FoundrySerialInvalid => TransactionFailureReason::FoundrySerialInvalid,
            ProtoState::DelegationCommitmentInputMissing => TransactionFailureReason::DelegationCommitmentInputMissing,
            ProtoState::DelegationRewardInputMissing => TransactionFailureReason::DelegationRewardInputMissing,
            ProtoState::DelegationRewardsClaimingInvalid => TransactionFailureReason::DelegationRewardsClaimingInvalid,
            ProtoState::DelegationOutputTransitionedTwice => {
                TransactionFailureReason::DelegationOutputTransitionedTwice
            }
            ProtoState::DelegationModified => TransactionFailureReason::DelegationModified,
            ProtoState::DelegationStartEpochInvalid => TransactionFailureReason::DelegationStartEpochInvalid,
            ProtoState::DelegationAmountMismatch => TransactionFailureReason::DelegationAmountMismatch,
            ProtoState::DelegationEndEpochNotZero => TransactionFailureReason::DelegationEndEpochNotZero,
            ProtoState::DelegationEndEpochInvalid => TransactionFailureReason::DelegationEndEpochInvalid,
            ProtoState::CapabilitiesNativeTokenBurningNotAllowed => {
                TransactionFailureReason::CapabilitiesNativeTokenBurningNotAllowed
            }
            ProtoState::CapabilitiesManaBurningNotAllowed => {
                TransactionFailureReason::CapabilitiesManaBurningNotAllowed
            }
            ProtoState::CapabilitiesAccountDestructionNotAllowed => {
                TransactionFailureReason::CapabilitiesAccountDestructionNotAllowed
            }
            ProtoState::CapabilitiesAnchorDestructionNotAllowed => {
                TransactionFailureReason::CapabilitiesAnchorDestructionNotAllowed
            }
            ProtoState::CapabilitiesFoundryDestructionNotAllowed => {
                TransactionFailureReason::CapabilitiesFoundryDestructionNotAllowed
            }
            ProtoState::CapabilitiesNftDestructionNotAllowed => {
                TransactionFailureReason::CapabilitiesNftDestructionNotAllowed
            }
            ProtoState::SemanticValidationFailed => TransactionFailureReason::SemanticValidationFailed,
        })
    }
}
