// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::semantic as bee;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ConflictReason {
    None = 0,
    InputUtxoAlreadySpent = 1,
    InputUtxoAlreadySpentInThisMilestone = 2,
    InputUtxoNotFound = 3,
    CreatedConsumedAmountMismatch = 4,
    InvalidSignature = 5,
    TimelockNotExpired = 6,
    InvalidNativeTokens = 7,
    StorageDepositReturnUnfulfilled = 8,
    InvalidUnlock = 9,
    InputsCommitmentsMismatch = 10,
    UnverifiedSender = 11,
    InvalidChainStateTransition = 12,
    SemanticValidationFailed = 255,
}

impl From<bee::ConflictReason> for ConflictReason {
    fn from(value: bee::ConflictReason) -> Self {
        match value {
            bee::ConflictReason::None => Self::None,
            bee::ConflictReason::InputUtxoAlreadySpent => Self::InputUtxoAlreadySpent,
            bee::ConflictReason::InputUtxoAlreadySpentInThisMilestone => Self::InputUtxoAlreadySpentInThisMilestone,
            bee::ConflictReason::InputUtxoNotFound => Self::InputUtxoNotFound,
            bee::ConflictReason::CreatedConsumedAmountMismatch => Self::CreatedConsumedAmountMismatch,
            bee::ConflictReason::InvalidSignature => Self::InvalidSignature,
            bee::ConflictReason::TimelockNotExpired => Self::TimelockNotExpired,
            bee::ConflictReason::InvalidNativeTokens => Self::InvalidNativeTokens,
            bee::ConflictReason::StorageDepositReturnUnfulfilled => Self::StorageDepositReturnUnfulfilled,
            bee::ConflictReason::InvalidUnlock => Self::InvalidUnlock,
            bee::ConflictReason::InputsCommitmentsMismatch => Self::InputsCommitmentsMismatch,
            bee::ConflictReason::UnverifiedSender => Self::UnverifiedSender,
            bee::ConflictReason::InvalidChainStateTransition => Self::InvalidChainStateTransition,
            bee::ConflictReason::SemanticValidationFailed => Self::SemanticValidationFailed,
        }
    }
}

impl From<ConflictReason> for bee::ConflictReason {
    fn from(value: ConflictReason) -> Self {
        match value {
            ConflictReason::None => Self::None,
            ConflictReason::InputUtxoAlreadySpent => Self::InputUtxoAlreadySpent,
            ConflictReason::InputUtxoAlreadySpentInThisMilestone => Self::InputUtxoAlreadySpentInThisMilestone,
            ConflictReason::InputUtxoNotFound => Self::InputUtxoNotFound,
            ConflictReason::CreatedConsumedAmountMismatch => Self::CreatedConsumedAmountMismatch,
            ConflictReason::InvalidSignature => Self::InvalidSignature,
            ConflictReason::TimelockNotExpired => Self::TimelockNotExpired,
            ConflictReason::InvalidNativeTokens => Self::InvalidNativeTokens,
            ConflictReason::StorageDepositReturnUnfulfilled => Self::StorageDepositReturnUnfulfilled,
            ConflictReason::InvalidUnlock => Self::InvalidUnlock,
            ConflictReason::InputsCommitmentsMismatch => Self::InputsCommitmentsMismatch,
            ConflictReason::UnverifiedSender => Self::UnverifiedSender,
            ConflictReason::InvalidChainStateTransition => Self::InvalidChainStateTransition,
            ConflictReason::SemanticValidationFailed => Self::SemanticValidationFailed,
        }
    }
}

#[cfg(feature = "inx")]
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
