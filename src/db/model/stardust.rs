// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust as stardust;
use serde::{Deserialize, Serialize};

use super::Model;

#[allow(missing_docs)]
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub enum LedgerInclusionState {
    NoTransaction,
    Included,
    Conflicting,
}

impl From<inx::LedgerInclusionState> for LedgerInclusionState {
    fn from(value: inx::LedgerInclusionState) -> Self {
        match value {
            inx::LedgerInclusionState::NoTransaction => Self::NoTransaction,
            inx::LedgerInclusionState::Included => Self::Included,
            inx::LedgerInclusionState::Conflicting => Self::Conflicting,
        }
    }
}

#[allow(missing_docs)]
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub enum ConflictReason {
    None,
    InputAlreadySpent,
    InputAlreadySpentInThisMilestone,
    InputNotFound,
    InputOutputSumMismatch,
    InvalidSignature,
    InvalidNetworkId,
    SemanticValidationFailed,
}

impl From<inx::ConflictReason> for ConflictReason {
    fn from(value: inx::ConflictReason) -> Self {
        match value {
            inx::ConflictReason::None => Self::None,
            inx::ConflictReason::InputAlreadySpent => Self::InputAlreadySpent,
            inx::ConflictReason::InputAlreadySpentInThisMilestone => Self::InputAlreadySpentInThisMilestone,
            inx::ConflictReason::InputNotFound => Self::InputNotFound,
            inx::ConflictReason::InputOutputSumMismatch => Self::InputOutputSumMismatch,
            inx::ConflictReason::InvalidSignature => Self::InvalidSignature,
            inx::ConflictReason::InvalidNetworkId => Self::InvalidNetworkId,
            inx::ConflictReason::SemanticValidationFailed => Self::SemanticValidationFailed,
        }
    }
}

/// Model for the metadata of a [`Message`].
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    /// Status of the solidification process.
    pub is_solid: bool,
    /// Indicates that the message should be promoted.
    pub should_promote: bool,
    /// Indicates that the message should be reattached.
    pub should_reattach: bool,
    /// The milestone that referenced the message.
    pub referenced_by_milestone_index: u32,
    /// The corresponding milestone index.
    pub milestone_index: u32,
    /// Indicates if a message is part of the ledger state or not.
    pub ledger_inclusion_state: LedgerInclusionState,
    /// Indicates if a conflict occured, and if so holds information about the reason for the conflict.
    pub conflict_reason: ConflictReason,
}

/// Model for the [`Message`].
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The [`MessageId`](stardust::MessageId) of the message.
    #[serde(rename = "_id")]
    pub message_id: stardust::MessageId,
    /// The actual rich representation of the [`Message`](stardust::Message).
    pub message: stardust::Message,
    /// The raw bytes of the message.
    pub raw: Vec<u8>,
    /// The metadata associated with a message.
    pub metadata: Option<Metadata>,
}

impl Model for Message {
    const COLLECTION: &'static str = "stardust_messages";

    type Id = stardust::MessageId;
}

/// Model for the [`Milestone`]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    /// The milestone index.
    #[serde(rename = "_id")]
    pub milestone_index: u32,
    /// The timestamp of the milestone.
    pub milestone_timestamp: mongodb::bson::DateTime,
    /// The [`MessageId`](stardust::MessageId) of the milestone.
    pub message_id: stardust::MessageId,
    /// The [`MilestoneId`](stardust::payload::milestone::MilestoneId) of the milestone.
    pub milestone_id: stardust::payload::milestone::MilestoneId,
}

impl Model for Milestone {
    const COLLECTION: &'static str = "stardust_milestones";

    type Id = u32;
}
