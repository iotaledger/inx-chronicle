// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::{semantic::ConflictReason, Message, MessageId};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::db::model::{inclusion_state::LedgerInclusionState, Model};
/// Chronicle Message record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageRecord {
    /// The message ID.
    pub message_id: MessageId,
    /// The message.
    pub message: Message,
    /// The raw bytes of the message.
    pub raw: Vec<u8>,
    /// The message's metadata.
    pub metadata: Option<MessageMetadata>,
}

impl MessageRecord {
    /// Creates a new message record.
    pub fn new(message: Message, raw: Vec<u8>) -> Self {
        Self {
            message_id: message.id(),
            message,
            raw,
            metadata: None,
        }
    }
    /// Returns Message id of the message.
    pub fn message_id(&self) -> &MessageId {
        &self.message_id
    }

    /// Returns the message.
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// Returns the message metadata.
    pub fn metadata(&self) -> Option<&MessageMetadata> {
        self.metadata.as_ref()
    }
}

impl Model for MessageRecord {
    const COLLECTION: &'static str = "stardust_messages";

    fn key(&self) -> mongodb::bson::Document {
        doc! { "message_id": self.message_id.to_string() }
    }
}

impl TryFrom<inx::proto::Message> for MessageRecord {
    type Error = inx::Error;

    fn try_from(value: inx::proto::Message) -> Result<Self, Self::Error> {
        let (message, raw_message) = value.try_into()?;
        Ok(Self::new(message.message, raw_message))
    }
}

impl TryFrom<(inx::proto::RawMessage, inx::proto::MessageMetadata)> for MessageRecord {
    type Error = inx::Error;

    fn try_from(
        (raw_message, metadata): (inx::proto::RawMessage, inx::proto::MessageMetadata),
    ) -> Result<Self, Self::Error> {
        let message = Message::try_from(raw_message.clone())?;
        Ok(Self {
            message_id: message.id(),
            message,
            raw: raw_message.data,
            metadata: Some(inx::MessageMetadata::try_from(metadata)?.into()),
        })
    }
}

/// Message metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Status of the solidification process.
    pub is_solid: bool,
    /// Indicates that the message should be promoted.
    pub should_promote: bool,
    /// Indicates that the message should be reattached.
    pub should_reattach: bool,
    /// The milestone index referencing the message.
    pub referenced_by_milestone_index: u32,
    /// The corresponding milestone index.
    pub milestone_index: u32,
    /// The inclusion state of the message.
    pub inclusion_state: LedgerInclusionState,
    /// If the ledger inclusion state is conflicting, the reason for the conflict.
    pub conflict_reason: Option<ConflictReason>,
}

impl MessageMetadata {
    /// Creates a new message metadata.
    pub fn new(
        is_solid: bool,
        should_promote: bool,
        should_reattach: bool,
        referenced_by_milestone_index: u32,
        milestone_index: u32,
        inclusion_state: LedgerInclusionState,
        conflict_reason: Option<ConflictReason>,
    ) -> Self {
        Self {
            is_solid,
            should_promote,
            should_reattach,
            referenced_by_milestone_index,
            milestone_index,
            inclusion_state,
            conflict_reason,
        }
    }

    /// Returns the solidification status.
    pub fn is_solid(&self) -> bool {
        self.is_solid
    }

    /// Returns should promote indicator.
    pub fn should_promote(&self) -> bool {
        self.should_promote
    }

    /// Returns should reattach indicator.
    pub fn should_reattach(&self) -> bool {
        self.should_reattach
    }

    /// Returns the milestone index referencing the message.
    pub fn referenced_by_milestone_index(&self) -> u32 {
        self.referenced_by_milestone_index
    }

    /// Returns the corresponding milestone index.
    pub fn milestone_index(&self) -> u32 {
        self.milestone_index
    }

    /// Returns the inclusion state of the message.
    pub fn inclusion_state(&self) -> &LedgerInclusionState {
        &self.inclusion_state
    }

    /// Returns the reason for the conflict.
    pub fn conflict_reason(&self) -> Option<&ConflictReason> {
        self.conflict_reason.as_ref()
    }
}

impl From<inx::MessageMetadata> for MessageMetadata {
    fn from(metadata: inx::MessageMetadata) -> Self {
        Self {
            is_solid: metadata.is_solid,
            should_promote: metadata.should_promote,
            should_reattach: metadata.should_reattach,
            referenced_by_milestone_index: metadata.referenced_by_milestone_index,
            milestone_index: metadata.milestone_index,
            inclusion_state: match metadata.ledger_inclusion_state {
                inx::LedgerInclusionState::Included => LedgerInclusionState::Included,
                inx::LedgerInclusionState::NoTransaction => LedgerInclusionState::NoTransaction,
                inx::LedgerInclusionState::Conflicting => LedgerInclusionState::Conflicting,
            },
            conflict_reason: match metadata.conflict_reason {
                inx::ConflictReason::None => None,
                inx::ConflictReason::InputAlreadySpent => Some(ConflictReason::InputUtxoAlreadySpent),
                inx::ConflictReason::InputAlreadySpentInThisMilestone => {
                    Some(ConflictReason::InputUtxoAlreadySpentInThisMilestone)
                }
                inx::ConflictReason::InputNotFound => Some(ConflictReason::InputUtxoNotFound),
                inx::ConflictReason::InputOutputSumMismatch => todo!(),
                inx::ConflictReason::InvalidSignature => Some(ConflictReason::InvalidSignature),
                inx::ConflictReason::InvalidNetworkId => todo!(),
                inx::ConflictReason::SemanticValidationFailed => Some(ConflictReason::SemanticValidationFailed),
            },
        }
    }
}
