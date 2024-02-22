// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing block metadata types.

use iota_sdk::{
    types::{
        api::core::BlockFailureReason,
        block::{
            self as iota, payload::signed_transaction::TransactionId, semantic::TransactionFailureReason, BlockId,
        },
    },
    utils::serde::option_string,
};
use serde::{Deserialize, Serialize};

use super::raw::Raw;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockMetadata {
    pub block_id: BlockId,
    pub block_state: BlockState,
    #[serde(with = "option_string")]
    pub block_failure_reason: Option<BlockFailureReason>,
    pub transaction_metadata: Option<TransactionMetadata>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]

pub struct TransactionMetadata {
    pub transaction_id: TransactionId,
    pub transaction_state: TransactionState,
    #[serde(with = "option_string")]
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
