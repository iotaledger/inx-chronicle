// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::{semantic::ConflictReason, Message, MessageId};
use mongodb::bson::doc;
use packable::PackableExt;
use serde::{Deserialize, Serialize};

use crate::db::model::{inclusion_state::LedgerInclusionState, InxConversionError, Model};
/// Chronicle Message record
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MessageRecord {
    pub message_id: MessageId,
    pub message: Message,
    pub raw: Vec<u8>,
    pub milestone_index: Option<u32>,
    pub inclusion_state: Option<LedgerInclusionState>,
    pub conflict_reason: Option<ConflictReason>,
}

#[allow(unused)]
impl MessageRecord {
    /// Create new message record
    pub fn new(message_id: MessageId, message: Message, raw: Vec<u8>) -> Self {
        Self {
            message_id,
            message,
            raw,
            milestone_index: None,
            inclusion_state: None,
            conflict_reason: None,
        }
    }
    /// Return Message id of the message
    pub fn message_id(&self) -> &MessageId {
        &self.message_id
    }

    /// Return the message
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// Return referenced milestone index
    pub fn milestone_index(&self) -> Option<u32> {
        self.milestone_index
    }

    /// Return inclusion_state
    pub fn inclusion_state(&self) -> Option<&LedgerInclusionState> {
        self.inclusion_state.as_ref()
    }

    /// Return conflict_reason
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
    type Error = InxConversionError;

    fn try_from(value: inx::proto::Message) -> Result<Self, Self::Error> {
        let message_id = message_id_from_inx(value.message_id.ok_or(InxConversionError::MissingField("message_id"))?)?;
        let raw_message = value.message.ok_or(InxConversionError::MissingField("message"))?.data;
        Ok(Self::new(
            message_id,
            bincode::deserialize(&raw_message).map_err(|e| InxConversionError::PackableError(Box::new(e)))?,
            // Message::unpack_verified(&raw_message).map_err(|e| InxConversionError::PackableError(Box::new(e)))?,
            raw_message,
        ))
    }
}

pub(crate) fn message_id_from_inx(value: inx::proto::MessageId) -> Result<MessageId, InxConversionError> {
    Ok(MessageId::from(
        <[u8; MessageId::LENGTH]>::try_from(value.id).map_err(|_| InxConversionError::InvalidBufferLength)?,
    ))
}
