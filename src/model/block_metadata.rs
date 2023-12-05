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
pub enum BlockFailureReason {
    /// The block is too old to issue.
    TooOldToIssue = 1,
    /// One of the block's parents is too old.
    ParentTooOld = 2,
    /// One of the block's parents does not exist.
    ParentDoesNotExist = 3,
    /// One of the block's parents is invalid.
    ParentInvalid = 4,
    /// The block's issuer account could not be found.
    IssuerAccountNotFound = 5,
    /// The block's protocol version is invalid.
    VersionInvalid = 6,
    /// The mana cost could not be calculated.
    ManaCostCalculationFailed = 7,
    /// The block's issuer account burned insufficient Mana for a block.
    BurnedInsufficientMana = 8,
    /// The account is invalid.
    AccountInvalid = 9,
    /// The block's signature is invalid.
    SignatureInvalid = 10,
    /// The block is dropped due to congestion.
    DroppedDueToCongestion = 11,
    /// The block payload is invalid.
    PayloadInvalid = 12,
    /// The block is invalid.
    Invalid = 255,
}

impl From<BlockFailureReason> for iota_sdk::types::api::core::BlockFailureReason {
    fn from(value: BlockFailureReason) -> Self {
        match value {
            BlockFailureReason::TooOldToIssue => Self::TooOldToIssue,
            BlockFailureReason::ParentTooOld => Self::ParentTooOld,
            BlockFailureReason::ParentDoesNotExist => Self::ParentDoesNotExist,
            BlockFailureReason::ParentInvalid => Self::ParentInvalid,
            BlockFailureReason::IssuerAccountNotFound => Self::IssuerAccountNotFound,
            BlockFailureReason::VersionInvalid => Self::VersionInvalid,
            BlockFailureReason::ManaCostCalculationFailed => Self::ManaCostCalculationFailed,
            BlockFailureReason::BurnedInsufficientMana => Self::BurnedInsufficientMana,
            BlockFailureReason::AccountInvalid => Self::AccountInvalid,
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
pub enum TransactionFailureReason {
    /// The referenced UTXO was already spent.
    InputUtxoAlreadySpent = 1,
    /// The transaction is conflicting with another transaction. Conflicting specifically means a double spend
    /// situation that both transaction pass all validation rules, eventually losing one(s) should have this reason.
    ConflictingWithAnotherTx = 2,
    /// The referenced UTXO is invalid.
    InvalidReferencedUtxo = 3,
    /// The transaction is invalid.
    InvalidTransaction = 4,
    /// The sum of the inputs and output base token amount does not match.
    SumInputsOutputsAmountMismatch = 5,
    /// The unlock block signature is invalid.
    InvalidUnlockBlockSignature = 6,
    /// The configured timelock is not yet expired.
    TimelockNotExpired = 7,
    /// The given native tokens are invalid.
    InvalidNativeTokens = 8,
    /// The return amount in a transaction is not fulfilled by the output side.
    StorageDepositReturnUnfulfilled = 9,
    /// An input unlock was invalid.
    InvalidInputUnlock = 10,
    /// The output contains a Sender with an ident (address) which is not unlocked.
    SenderNotUnlocked = 11,
    /// The chain state transition is invalid.
    InvalidChainStateTransition = 12,
    /// The referenced input is created after transaction issuing time.
    InvalidTransactionIssuingTime = 13,
    /// The mana amount is invalid.
    InvalidManaAmount = 14,
    /// The Block Issuance Credits amount is invalid.
    InvalidBlockIssuanceCreditsAmount = 15,
    /// Reward Context Input is invalid.
    InvalidRewardContextInput = 16,
    /// Commitment Context Input is invalid.
    InvalidCommitmentContextInput = 17,
    /// Staking Feature is not provided in account output when claiming rewards.
    MissingStakingFeature = 18,
    /// Failed to claim staking reward.
    FailedToClaimStakingReward = 19,
    /// Failed to claim delegation reward.
    FailedToClaimDelegationReward = 20,
    /// Burning of native tokens is not allowed in the transaction capabilities.
    TransactionCapabilityNativeTokenBurningNotAllowed = 21,
    /// Burning of mana is not allowed in the transaction capabilities.
    TransactionCapabilityManaBurningNotAllowed = 22,
    /// Destruction of accounts is not allowed in the transaction capabilities.
    TransactionCapabilityAccountDestructionNotAllowed = 23,
    /// Destruction of anchors is not allowed in the transaction capabilities.
    TransactionCapabilityAnchorDestructionNotAllowed = 24,
    /// Destruction of foundries is not allowed in the transaction capabilities.
    TransactionCapabilityFoundryDestructionNotAllowed = 25,
    /// Destruction of nfts is not allowed in the transaction capabilities.
    TransactionCapabilityNftDestructionNotAllowed = 26,
    /// The semantic validation failed for a reason not covered by the previous variants.
    SemanticValidationFailed = 255,
}

impl From<TransactionFailureReason> for iota_sdk::types::block::semantic::TransactionFailureReason {
    fn from(value: TransactionFailureReason) -> Self {
        match value {
            TransactionFailureReason::InputUtxoAlreadySpent => Self::InputUtxoAlreadySpent,
            TransactionFailureReason::ConflictingWithAnotherTx => Self::ConflictingWithAnotherTx,
            TransactionFailureReason::InvalidReferencedUtxo => Self::InvalidReferencedUtxo,
            TransactionFailureReason::InvalidTransaction => Self::InvalidTransaction,
            TransactionFailureReason::SumInputsOutputsAmountMismatch => Self::SumInputsOutputsAmountMismatch,
            TransactionFailureReason::InvalidUnlockBlockSignature => Self::InvalidUnlockBlockSignature,
            TransactionFailureReason::TimelockNotExpired => Self::TimelockNotExpired,
            TransactionFailureReason::InvalidNativeTokens => Self::InvalidNativeTokens,
            TransactionFailureReason::StorageDepositReturnUnfulfilled => Self::StorageDepositReturnUnfulfilled,
            TransactionFailureReason::InvalidInputUnlock => Self::InvalidInputUnlock,
            TransactionFailureReason::SenderNotUnlocked => Self::SenderNotUnlocked,
            TransactionFailureReason::InvalidChainStateTransition => Self::InvalidChainStateTransition,
            TransactionFailureReason::InvalidTransactionIssuingTime => Self::InvalidTransactionIssuingTime,
            TransactionFailureReason::InvalidManaAmount => Self::InvalidManaAmount,
            TransactionFailureReason::InvalidBlockIssuanceCreditsAmount => Self::InvalidBlockIssuanceCreditsAmount,
            TransactionFailureReason::InvalidRewardContextInput => Self::InvalidRewardContextInput,
            TransactionFailureReason::InvalidCommitmentContextInput => Self::InvalidCommitmentContextInput,
            TransactionFailureReason::MissingStakingFeature => Self::MissingStakingFeature,
            TransactionFailureReason::FailedToClaimStakingReward => Self::FailedToClaimStakingReward,
            TransactionFailureReason::FailedToClaimDelegationReward => Self::FailedToClaimDelegationReward,
            TransactionFailureReason::TransactionCapabilityNativeTokenBurningNotAllowed => {
                Self::TransactionCapabilityNativeTokenBurningNotAllowed
            }
            TransactionFailureReason::TransactionCapabilityManaBurningNotAllowed => {
                Self::TransactionCapabilityManaBurningNotAllowed
            }
            TransactionFailureReason::TransactionCapabilityAccountDestructionNotAllowed => {
                Self::TransactionCapabilityAccountDestructionNotAllowed
            }
            TransactionFailureReason::TransactionCapabilityAnchorDestructionNotAllowed => {
                Self::TransactionCapabilityAnchorDestructionNotAllowed
            }
            TransactionFailureReason::TransactionCapabilityFoundryDestructionNotAllowed => {
                Self::TransactionCapabilityFoundryDestructionNotAllowed
            }
            TransactionFailureReason::TransactionCapabilityNftDestructionNotAllowed => {
                Self::TransactionCapabilityNftDestructionNotAllowed
            }
            TransactionFailureReason::SemanticValidationFailed => Self::SemanticValidationFailed,
        }
    }
}
