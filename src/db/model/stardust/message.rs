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
    /// The milestone index referencing the milestone.
    pub milestone_index: Option<u32>,
    /// The inclusion state of the message.
    pub inclusion_state: Option<LedgerInclusionState>,
    /// If the ledger inclusion state is conflicting, the reason for the conflict.
    pub conflict_reason: Option<ConflictReason>,
}

impl MessageRecord {
    /// Creates a new message record.
    pub fn new(message: Message, raw: Vec<u8>) -> Self {
        Self {
            message_id: message.id(),
            message,
            raw,
            milestone_index: None,
            inclusion_state: None,
            conflict_reason: None,
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

    /// Returns referenced milestone index.
    pub fn milestone_index(&self) -> Option<u32> {
        self.milestone_index
    }

    /// Returns inclusion_state.
    pub fn inclusion_state(&self) -> Option<&LedgerInclusionState> {
        self.inclusion_state.as_ref()
    }

    /// Returns conflict_reason.
    pub fn conflict_reason(&self) -> Option<&ConflictReason> {
        self.conflict_reason.as_ref()
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
