// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Display,
    ops::{
        Deref,
        DerefMut,
    },
    str::FromStr,
};

use anyhow::*;
use derive_more::From;
use serde::{
    Deserialize,
    Serialize,
};

use super::LedgerInclusionState;
use crate::stardust::semantic::ConflictReason;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, From, Hash, Ord, PartialOrd)]
pub enum MessageId {
    /// Chrysalis compatible message
    Chrysalis(bee_message_cpt2::MessageId),
    /// Stardust compatible message
    Stardust(bee_message_stardust::MessageId),
}

impl MessageId {
    pub fn is_null(&self) -> bool {
        match self {
            MessageId::Chrysalis(id) => id == &bee_message_cpt2::MessageId::null(),
            MessageId::Stardust(id) => id == &bee_message_stardust::MessageId::null(),
        }
    }
}

impl Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Chrysalis(id) => write!(f, "{}", id),
            Self::Stardust(id) => write!(f, "{}", id),
        }
    }
}

impl FromStr for MessageId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee_message_stardust::MessageId::from_str(s)
            .map(Self::Stardust)
            .or_else(|_| bee_message_cpt2::MessageId::from_str(s).map(Self::Chrysalis))?)
    }
}

/// Represent versioned message type.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, From)]
pub enum Message {
    /// Chrysalis compatible message
    Chrysalis(bee_message_cpt2::Message),
    /// Stardust compatible message
    Stardust(bee_message_stardust::Message),
}

impl Message {
    pub fn protocol_version(&self) -> u8 {
        match self {
            Message::Chrysalis(_) => 0,
            Message::Stardust(m) => m.protocol_version(),
        }
    }
}

impl std::convert::TryFrom<crate::cpt2::types::dtos::MessageDto> for Message {
    type Error = anyhow::Error;
    fn try_from(dto: crate::cpt2::types::dtos::MessageDto) -> Result<Self, Self::Error> {
        Ok(Self::Chrysalis(bee_message_cpt2::Message::try_from(&dto)?.into()))
    }
}

impl std::convert::TryFrom<crate::stardust::MessageDto> for Message {
    type Error = anyhow::Error;
    fn try_from(dto: crate::stardust::MessageDto) -> Result<Self, Self::Error> {
        Ok(Self::Stardust(bee_message_stardust::Message::try_from(&dto)?.into()))
    }
}

impl Message {
    /// Returns the message id
    pub fn id(&self) -> MessageId {
        match self {
            Self::Chrysalis(msg) => MessageId::Chrysalis(msg.id().0),
            Self::Stardust(msg) => MessageId::Stardust(msg.id()),
        }
    }
    /// Returns the parents of the message
    pub fn parents(&self) -> impl Iterator<Item = MessageId> + '_ {
        match self {
            Self::Chrysalis(msg) => {
                Box::new(msg.parents().iter().map(|p| MessageId::Chrysalis(*p))) as Box<dyn Iterator<Item = MessageId>>
            }
            Self::Stardust(msg) => {
                Box::new(msg.parents().iter().map(|p| MessageId::Stardust(*p))) as Box<dyn Iterator<Item = MessageId>>
            }
        }
    }
    /// Check if the message has milestone payload
    pub fn is_milestone(&self) -> bool {
        match self {
            Self::Chrysalis(msg) => {
                if let Some(bee_message_cpt2::payload::Payload::Milestone(_)) = msg.payload() {
                    return true;
                }
            }
            Self::Stardust(msg) => {
                if let Some(bee_message_stardust::payload::Payload::Milestone(_)) = msg.payload() {
                    return true;
                }
            }
        }
        false
    }
}
/// Chronicle Message record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageRecord {
    pub message_id: MessageId,
    // TODO: make use of protocol version to deserialize this
    pub message: Message,
    pub milestone_index: Option<u32>,
    pub inclusion_state: Option<LedgerInclusionState>,
    pub conflict_reason: Option<ConflictReason>,
    pub proof: Option<Proof>,
    pub protocol_version: u8,
}

impl MessageRecord {
    /// Create new message record
    pub fn new(message_id: MessageId, message: Message) -> Self {
        Self {
            message_id,
            protocol_version: message.protocol_version(),
            message,
            milestone_index: None,
            inclusion_state: None,
            conflict_reason: None,
            proof: None,
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
    /// Return proof
    pub fn proof(&self) -> Option<&Proof> {
        self.proof.as_ref()
    }

    /// Get the message's nonce
    pub fn nonce(&self) -> u64 {
        match &self.message {
            Message::Chrysalis(m) => m.nonce(),
            Message::Stardust(m) => m.nonce(),
        }
    }
}

impl Deref for MessageRecord {
    type Target = Message;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl DerefMut for MessageRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.message
    }
}

impl PartialOrd for MessageRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MessageRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.message_id.cmp(&other.message_id)
    }
}

impl PartialEq for MessageRecord {
    fn eq(&self, other: &Self) -> bool {
        self.message_id == other.message_id
    }
}
impl Eq for MessageRecord {}

impl From<(Message, crate::stardust::types::responses::MessageMetadataResponse)> for MessageRecord {
    fn from((message, metadata): (Message, crate::stardust::types::responses::MessageMetadataResponse)) -> Self {
        MessageRecord {
            message_id: message.id(),
            protocol_version: message.protocol_version(),
            message,
            milestone_index: metadata.referenced_by_milestone_index,
            inclusion_state: metadata.ledger_inclusion_state.map(Into::into),
            conflict_reason: metadata.conflict_reason.and_then(|c| c.try_into().ok()),
            proof: None,
        }
    }
}

impl From<(Message, crate::cpt2::types::responses::MessageMetadataResponse)> for MessageRecord {
    fn from((message, metadata): (Message, crate::cpt2::types::responses::MessageMetadataResponse)) -> Self {
        MessageRecord {
            message_id: message.id(),
            protocol_version: message.protocol_version(),
            message,
            milestone_index: metadata.referenced_by_milestone_index,
            inclusion_state: metadata.ledger_inclusion_state.map(Into::into),
            conflict_reason: metadata.conflict_reason.and_then(|c| c.try_into().ok()),
            proof: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Proof {
    milestone_index: u32,
    path: Vec<MessageId>,
}

impl Proof {
    pub fn new(milestone_index: u32, path: Vec<MessageId>) -> Self {
        Self { milestone_index, path }
    }
    pub fn milestone_index(&self) -> u32 {
        self.milestone_index
    }
    pub fn path(&self) -> &[MessageId] {
        &self.path
    }
    pub fn path_mut(&mut self) -> &mut Vec<MessageId> {
        &mut self.path
    }
}
