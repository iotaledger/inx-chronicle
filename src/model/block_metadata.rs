// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing block metadata types.

use iota_sdk::types::block::{self as iota, payload::signed_transaction::TransactionId, BlockId};
use serde::{Deserialize, Serialize};

use super::raw::Raw;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockMetadata {
    pub block_id: BlockId,
    pub block_state: BlockState,
    pub block_failure_reason: Option<BlockFailureReason>,
    pub transaction_metadata: Option<TransactionMetadata>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]

pub struct TransactionMetadata {
    pub transaction_id: TransactionId,
    pub transaction_state: TransactionState,
    pub transaction_failure_reason: Option<TransactionFailureReason>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockWithMetadata {
    pub metadata: BlockMetadata,
    pub block: Raw<iota::Block>,
}

/// Describes the state of a block.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockState {
    /// Stored but not confirmed.
    Pending,
    /// Acccepted.
    Accepted,
    /// Confirmed with the first level of knowledge.
    Confirmed,
    /// Included and can no longer be reverted.
    Finalized,
    /// Rejected by the node, and user should reissue payload if it contains one.
    Rejected,
    /// Not successfully issued due to failure reason.
    Failed,
    /// Unknown state.
    Unknown,
}

impl From<BlockState> for iota_sdk::types::api::core::BlockState {
    fn from(value: BlockState) -> Self {
        match value {
            BlockState::Pending => Self::Pending,
            BlockState::Accepted => Self::Pending,
            BlockState::Confirmed => Self::Confirmed,
            BlockState::Finalized => Self::Finalized,
            BlockState::Rejected => Self::Rejected,
            BlockState::Failed => Self::Failed,
            BlockState::Unknown => panic!("invalid block state"),
        }
    }
}

/// Describes the state of a transaction.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionState {
    /// Stored but not confirmed.
    Pending,
    /// Accepted.
    Accepted,
    /// Confirmed with the first level of knowledge.
    Confirmed,
    /// Included and can no longer be reverted.
    Finalized,
    /// The block is not successfully issued due to failure reason.
    Failed,
}

impl From<TransactionState> for iota_sdk::types::api::core::TransactionState {
    fn from(value: TransactionState) -> Self {
        match value {
            TransactionState::Pending => Self::Pending,
            TransactionState::Accepted => Self::Pending,
            TransactionState::Confirmed => Self::Confirmed,
            TransactionState::Finalized => Self::Finalized,
            TransactionState::Failed => Self::Failed,
        }
    }
}

/// Describes the reason of a block failure.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum BlockFailureReason {
    TooOldToIssue = 1,
    ParentTooOld = 2,
    ParentDoesNotExist = 3,
    IssuerAccountNotFound = 4,
    ManaCostCalculationFailed = 5,
    BurnedInsufficientMana = 6,
    AccountLocked = 7,
    AccountExpired = 8,
    SignatureInvalid = 9,
    DroppedDueToCongestion = 10,
    PayloadInvalid = 11,
    Invalid = 255,
}

impl From<BlockFailureReason> for iota_sdk::types::api::core::BlockFailureReason {
    fn from(value: BlockFailureReason) -> Self {
        match value {
            BlockFailureReason::TooOldToIssue => Self::TooOldToIssue,
            BlockFailureReason::ParentTooOld => Self::ParentTooOld,
            BlockFailureReason::ParentDoesNotExist => Self::ParentDoesNotExist,
            BlockFailureReason::IssuerAccountNotFound => Self::IssuerAccountNotFound,
            BlockFailureReason::ManaCostCalculationFailed => Self::ManaCostCalculationFailed,
            BlockFailureReason::BurnedInsufficientMana => Self::BurnedInsufficientMana,
            BlockFailureReason::AccountLocked => Self::AccountLocked,
            BlockFailureReason::AccountExpired => Self::AccountExpired,
            BlockFailureReason::SignatureInvalid => Self::SignatureInvalid,
            BlockFailureReason::DroppedDueToCongestion => Self::DroppedDueToCongestion,
            BlockFailureReason::PayloadInvalid => Self::PayloadInvalid,
            BlockFailureReason::Invalid => Self::Invalid,
        }
    }
}

/// Describes the reason of a transaction failure.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum TransactionFailureReason {
    ConflictRejected = 1,
    InputAlreadySpent = 2,
    InputCreationAfterTxCreation = 3,
    UnlockSignatureInvalid = 4,
    CommitmentInputReferenceInvalid = 5,
    BicInputReferenceInvalid = 6,
    RewardInputReferenceInvalid = 7,
    StakingRewardCalculationFailure = 8,
    DelegationRewardCalculationFailure = 9,
    InputOutputBaseTokenMismatch = 10,
    ManaOverflow = 11,
    InputOutputManaMismatch = 12,
    ManaDecayCreationIndexExceedsTargetIndex = 13,
    NativeTokenSumUnbalanced = 14,
    SimpleTokenSchemeMintedMeltedTokenDecrease = 15,
    SimpleTokenSchemeMintingInvalid = 16,
    SimpleTokenSchemeMeltingInvalid = 17,
    SimpleTokenSchemeMaximumSupplyChanged = 18,
    SimpleTokenSchemeGenesisInvalid = 19,
    MultiAddressLengthUnlockLengthMismatch = 20,
    MultiAddressUnlockThresholdNotReached = 21,
    SenderFeatureNotUnlocked = 22,
    IssuerFeatureNotUnlocked = 23,
    StakingRewardInputMissing = 24,
    StakingBlockIssuerFeatureMissing = 25,
    StakingCommitmentInputMissing = 26,
    StakingRewardClaimingInvalid = 27,
    StakingFeatureRemovedBeforeUnbonding = 28,
    StakingFeatureModifiedBeforeUnbonding = 29,
    StakingStartEpochInvalid = 30,
    StakingEndEpochTooEarly = 31,
    BlockIssuerCommitmentInputMissing = 32,
    BlockIssuanceCreditInputMissing = 33,
    BlockIssuerNotExpired = 34,
    BlockIssuerExpiryTooEarly = 35,
    ManaMovedOffBlockIssuerAccount = 36,
    AccountLocked = 37,
    TimelockCommitmentInputMissing = 38,
    TimelockNotExpired = 39,
    ExpirationCommitmentInputMissing = 40,
    ExpirationNotUnlockable = 41,
    ReturnAmountNotFulFilled = 42,
    NewChainOutputHasNonZeroedId = 43,
    ChainOutputImmutableFeaturesChanged = 44,
    ImplicitAccountDestructionDisallowed = 45,
    MultipleImplicitAccountCreationAddresses = 46,
    AccountInvalidFoundryCounter = 47,
    AnchorInvalidStateTransition = 48,
    AnchorInvalidGovernanceTransition = 49,
    FoundryTransitionWithoutAccount = 50,
    FoundrySerialInvalid = 51,
    DelegationCommitmentInputMissing = 52,
    DelegationRewardInputMissing = 53,
    DelegationRewardsClaimingInvalid = 54,
    DelegationOutputTransitionedTwice = 55,
    DelegationModified = 56,
    DelegationStartEpochInvalid = 57,
    DelegationAmountMismatch = 58,
    DelegationEndEpochNotZero = 59,
    DelegationEndEpochInvalid = 60,
    CapabilitiesNativeTokenBurningNotAllowed = 61,
    CapabilitiesManaBurningNotAllowed = 62,
    CapabilitiesAccountDestructionNotAllowed = 63,
    CapabilitiesAnchorDestructionNotAllowed = 64,
    CapabilitiesFoundryDestructionNotAllowed = 65,
    CapabilitiesNftDestructionNotAllowed = 66,
    SemanticValidationFailed = 255,
}

impl From<TransactionFailureReason> for iota_sdk::types::block::semantic::TransactionFailureReason {
    fn from(value: TransactionFailureReason) -> Self {
        match value {
            TransactionFailureReason::ConflictRejected => Self::ConflictRejected,
            TransactionFailureReason::InputAlreadySpent => Self::InputAlreadySpent,
            TransactionFailureReason::InputCreationAfterTxCreation => Self::InputCreationAfterTxCreation,
            TransactionFailureReason::UnlockSignatureInvalid => Self::UnlockSignatureInvalid,
            TransactionFailureReason::CommitmentInputReferenceInvalid => Self::CommitmentInputReferenceInvalid,
            TransactionFailureReason::BicInputReferenceInvalid => Self::BicInputReferenceInvalid,
            TransactionFailureReason::RewardInputReferenceInvalid => Self::RewardInputReferenceInvalid,
            TransactionFailureReason::StakingRewardCalculationFailure => Self::StakingRewardCalculationFailure,
            TransactionFailureReason::DelegationRewardCalculationFailure => Self::DelegationRewardCalculationFailure,
            TransactionFailureReason::InputOutputBaseTokenMismatch => Self::InputOutputBaseTokenMismatch,
            TransactionFailureReason::ManaOverflow => Self::ManaOverflow,
            TransactionFailureReason::InputOutputManaMismatch => Self::InputOutputManaMismatch,
            TransactionFailureReason::ManaDecayCreationIndexExceedsTargetIndex => {
                Self::ManaDecayCreationIndexExceedsTargetIndex
            }
            TransactionFailureReason::NativeTokenSumUnbalanced => Self::NativeTokenSumUnbalanced,
            TransactionFailureReason::SimpleTokenSchemeMintedMeltedTokenDecrease => {
                Self::SimpleTokenSchemeMintedMeltedTokenDecrease
            }
            TransactionFailureReason::SimpleTokenSchemeMintingInvalid => Self::SimpleTokenSchemeMintingInvalid,
            TransactionFailureReason::SimpleTokenSchemeMeltingInvalid => Self::SimpleTokenSchemeMeltingInvalid,
            TransactionFailureReason::SimpleTokenSchemeMaximumSupplyChanged => {
                Self::SimpleTokenSchemeMaximumSupplyChanged
            }
            TransactionFailureReason::SimpleTokenSchemeGenesisInvalid => Self::SimpleTokenSchemeGenesisInvalid,
            TransactionFailureReason::MultiAddressLengthUnlockLengthMismatch => {
                Self::MultiAddressLengthUnlockLengthMismatch
            }
            TransactionFailureReason::MultiAddressUnlockThresholdNotReached => {
                Self::MultiAddressUnlockThresholdNotReached
            }
            TransactionFailureReason::SenderFeatureNotUnlocked => Self::SenderFeatureNotUnlocked,
            TransactionFailureReason::IssuerFeatureNotUnlocked => Self::IssuerFeatureNotUnlocked,
            TransactionFailureReason::StakingRewardInputMissing => Self::StakingRewardInputMissing,
            TransactionFailureReason::StakingBlockIssuerFeatureMissing => Self::StakingBlockIssuerFeatureMissing,
            TransactionFailureReason::StakingCommitmentInputMissing => Self::StakingCommitmentInputMissing,
            TransactionFailureReason::StakingRewardClaimingInvalid => Self::StakingRewardClaimingInvalid,
            TransactionFailureReason::StakingFeatureRemovedBeforeUnbonding => {
                Self::StakingFeatureRemovedBeforeUnbonding
            }
            TransactionFailureReason::StakingFeatureModifiedBeforeUnbonding => {
                Self::StakingFeatureModifiedBeforeUnbonding
            }
            TransactionFailureReason::StakingStartEpochInvalid => Self::StakingStartEpochInvalid,
            TransactionFailureReason::StakingEndEpochTooEarly => Self::StakingEndEpochTooEarly,
            TransactionFailureReason::BlockIssuerCommitmentInputMissing => Self::BlockIssuerCommitmentInputMissing,
            TransactionFailureReason::BlockIssuanceCreditInputMissing => Self::BlockIssuanceCreditInputMissing,
            TransactionFailureReason::BlockIssuerNotExpired => Self::BlockIssuerNotExpired,
            TransactionFailureReason::BlockIssuerExpiryTooEarly => Self::BlockIssuerExpiryTooEarly,
            TransactionFailureReason::ManaMovedOffBlockIssuerAccount => Self::ManaMovedOffBlockIssuerAccount,
            TransactionFailureReason::AccountLocked => Self::AccountLocked,
            TransactionFailureReason::TimelockCommitmentInputMissing => Self::TimelockCommitmentInputMissing,
            TransactionFailureReason::TimelockNotExpired => Self::TimelockNotExpired,
            TransactionFailureReason::ExpirationCommitmentInputMissing => Self::ExpirationCommitmentInputMissing,
            TransactionFailureReason::ExpirationNotUnlockable => Self::ExpirationNotUnlockable,
            TransactionFailureReason::ReturnAmountNotFulFilled => Self::ReturnAmountNotFulFilled,
            TransactionFailureReason::NewChainOutputHasNonZeroedId => Self::NewChainOutputHasNonZeroedId,
            TransactionFailureReason::ChainOutputImmutableFeaturesChanged => Self::ChainOutputImmutableFeaturesChanged,
            TransactionFailureReason::ImplicitAccountDestructionDisallowed => {
                Self::ImplicitAccountDestructionDisallowed
            }
            TransactionFailureReason::MultipleImplicitAccountCreationAddresses => {
                Self::MultipleImplicitAccountCreationAddresses
            }
            TransactionFailureReason::AccountInvalidFoundryCounter => Self::AccountInvalidFoundryCounter,
            TransactionFailureReason::AnchorInvalidStateTransition => Self::AnchorInvalidStateTransition,
            TransactionFailureReason::AnchorInvalidGovernanceTransition => Self::AnchorInvalidGovernanceTransition,
            TransactionFailureReason::FoundryTransitionWithoutAccount => Self::FoundryTransitionWithoutAccount,
            TransactionFailureReason::FoundrySerialInvalid => Self::FoundrySerialInvalid,
            TransactionFailureReason::DelegationCommitmentInputMissing => Self::DelegationCommitmentInputMissing,
            TransactionFailureReason::DelegationRewardInputMissing => Self::DelegationRewardInputMissing,
            TransactionFailureReason::DelegationRewardsClaimingInvalid => Self::DelegationRewardsClaimingInvalid,
            TransactionFailureReason::DelegationOutputTransitionedTwice => Self::DelegationOutputTransitionedTwice,
            TransactionFailureReason::DelegationModified => Self::DelegationModified,
            TransactionFailureReason::DelegationStartEpochInvalid => Self::DelegationStartEpochInvalid,
            TransactionFailureReason::DelegationAmountMismatch => Self::DelegationAmountMismatch,
            TransactionFailureReason::DelegationEndEpochNotZero => Self::DelegationEndEpochNotZero,
            TransactionFailureReason::DelegationEndEpochInvalid => Self::DelegationEndEpochInvalid,
            TransactionFailureReason::CapabilitiesNativeTokenBurningNotAllowed => {
                Self::CapabilitiesNativeTokenBurningNotAllowed
            }
            TransactionFailureReason::CapabilitiesManaBurningNotAllowed => Self::CapabilitiesManaBurningNotAllowed,
            TransactionFailureReason::CapabilitiesAccountDestructionNotAllowed => {
                Self::CapabilitiesAccountDestructionNotAllowed
            }
            TransactionFailureReason::CapabilitiesAnchorDestructionNotAllowed => {
                Self::CapabilitiesAnchorDestructionNotAllowed
            }
            TransactionFailureReason::CapabilitiesFoundryDestructionNotAllowed => {
                Self::CapabilitiesFoundryDestructionNotAllowed
            }
            TransactionFailureReason::CapabilitiesNftDestructionNotAllowed => {
                Self::CapabilitiesNftDestructionNotAllowed
            }
            TransactionFailureReason::SemanticValidationFailed => Self::SemanticValidationFailed,
        }
    }
}
