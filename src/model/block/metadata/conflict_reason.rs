// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::semantic as iota;
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

impl From<iota::ConflictReason> for ConflictReason {
    fn from(value: iota::ConflictReason) -> Self {
        match value {
            iota::ConflictReason::None => Self::None,
            iota::ConflictReason::InputUtxoAlreadySpent => Self::InputUtxoAlreadySpent,
            iota::ConflictReason::InputUtxoAlreadySpentInThisMilestone => Self::InputUtxoAlreadySpentInThisMilestone,
            iota::ConflictReason::InputUtxoNotFound => Self::InputUtxoNotFound,
            iota::ConflictReason::CreatedConsumedAmountMismatch => Self::CreatedConsumedAmountMismatch,
            iota::ConflictReason::InvalidSignature => Self::InvalidSignature,
            iota::ConflictReason::TimelockNotExpired => Self::TimelockNotExpired,
            iota::ConflictReason::InvalidNativeTokens => Self::InvalidNativeTokens,
            iota::ConflictReason::StorageDepositReturnUnfulfilled => Self::StorageDepositReturnUnfulfilled,
            iota::ConflictReason::InvalidUnlock => Self::InvalidUnlock,
            iota::ConflictReason::InputsCommitmentsMismatch => Self::InputsCommitmentsMismatch,
            iota::ConflictReason::UnverifiedSender => Self::UnverifiedSender,
            iota::ConflictReason::InvalidChainStateTransition => Self::InvalidChainStateTransition,
            iota::ConflictReason::SemanticValidationFailed => Self::SemanticValidationFailed,
        }
    }
}

impl From<ConflictReason> for iota::ConflictReason {
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
