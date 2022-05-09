// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::semantic as stardust;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
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
    InvalidUnlockBlock = 9,
    InputsCommitmentsMismatch = 10,
    UnverifiedSender = 11,
    InvalidChainStateTransition = 12,
    SemanticValidationFailed = 255,
}

impl From<stardust::ConflictReason> for ConflictReason {
    fn from(value: stardust::ConflictReason) -> Self {
        match value {
            stardust::ConflictReason::None => ConflictReason::None,
            stardust::ConflictReason::InputUtxoAlreadySpent => ConflictReason::InputUtxoAlreadySpent,
            stardust::ConflictReason::InputUtxoAlreadySpentInThisMilestone => {
                ConflictReason::InputUtxoAlreadySpentInThisMilestone
            }
            stardust::ConflictReason::InputUtxoNotFound => ConflictReason::InputUtxoNotFound,
            stardust::ConflictReason::CreatedConsumedAmountMismatch => ConflictReason::CreatedConsumedAmountMismatch,
            stardust::ConflictReason::InvalidSignature => ConflictReason::InvalidSignature,
            stardust::ConflictReason::TimelockNotExpired => ConflictReason::TimelockNotExpired,
            stardust::ConflictReason::InvalidNativeTokens => ConflictReason::InvalidNativeTokens,
            stardust::ConflictReason::StorageDepositReturnUnfulfilled => {
                ConflictReason::StorageDepositReturnUnfulfilled
            }
            stardust::ConflictReason::InvalidUnlockBlock => ConflictReason::InvalidUnlockBlock,
            stardust::ConflictReason::InputsCommitmentsMismatch => ConflictReason::InputsCommitmentsMismatch,
            stardust::ConflictReason::UnverifiedSender => ConflictReason::UnverifiedSender,
            stardust::ConflictReason::InvalidChainStateTransition => ConflictReason::InvalidChainStateTransition,
            stardust::ConflictReason::SemanticValidationFailed => ConflictReason::SemanticValidationFailed,
        }
    }
}

impl From<ConflictReason> for stardust::ConflictReason {
    fn from(value: ConflictReason) -> Self {
        match value {
            ConflictReason::None => stardust::ConflictReason::None,
            ConflictReason::InputUtxoAlreadySpent => stardust::ConflictReason::InputUtxoAlreadySpent,
            ConflictReason::InputUtxoAlreadySpentInThisMilestone => {
                stardust::ConflictReason::InputUtxoAlreadySpentInThisMilestone
            }
            ConflictReason::InputUtxoNotFound => stardust::ConflictReason::InputUtxoNotFound,
            ConflictReason::CreatedConsumedAmountMismatch => stardust::ConflictReason::CreatedConsumedAmountMismatch,
            ConflictReason::InvalidSignature => stardust::ConflictReason::InvalidSignature,
            ConflictReason::TimelockNotExpired => stardust::ConflictReason::TimelockNotExpired,
            ConflictReason::InvalidNativeTokens => stardust::ConflictReason::InvalidNativeTokens,
            ConflictReason::StorageDepositReturnUnfulfilled => {
                stardust::ConflictReason::StorageDepositReturnUnfulfilled
            }
            ConflictReason::InvalidUnlockBlock => stardust::ConflictReason::InvalidUnlockBlock,
            ConflictReason::InputsCommitmentsMismatch => stardust::ConflictReason::InputsCommitmentsMismatch,
            ConflictReason::UnverifiedSender => stardust::ConflictReason::UnverifiedSender,
            ConflictReason::InvalidChainStateTransition => stardust::ConflictReason::InvalidChainStateTransition,
            ConflictReason::SemanticValidationFailed => stardust::ConflictReason::SemanticValidationFailed,
        }
    }
}
