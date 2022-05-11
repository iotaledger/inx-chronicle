// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod address;
mod input;
mod output;
mod payload;
mod signature;
mod unlock_block;

use bee_message_stardust as stardust;
use serde::{Deserialize, Serialize};

pub use self::{address::*, input::*, output::*, payload::*, signature::*, unlock_block::*};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Hash, Ord, PartialOrd, Eq)]
#[serde(transparent)]
pub struct MessageId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl MessageId {
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<stardust::MessageId> for MessageId {
    fn from(value: stardust::MessageId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<MessageId> for stardust::MessageId {
    type Error = crate::types::error::Error;

    fn try_from(value: MessageId) -> Result<Self, Self::Error> {
        Ok(stardust::MessageId::new(value.0.as_ref().try_into()?))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub protocol_version: u8,
    pub parents: Box<[MessageId]>,
    pub payload: Option<Payload>,
    #[serde(with = "crate::types::stringify")]
    pub nonce: u64,
}

impl From<stardust::Message> for Message {
    fn from(value: stardust::Message) -> Self {
        Self {
            id: value.id().into(),
            protocol_version: value.protocol_version(),
            parents: value.parents().iter().map(|id| MessageId::from(*id)).collect(),
            payload: value.payload().map(Into::into),
            nonce: value.nonce(),
        }
    }
}

impl TryFrom<Message> for stardust::Message {
    type Error = crate::types::error::Error;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        let mut builder = stardust::MessageBuilder::<u64>::new(stardust::parent::Parents::new(
            Vec::from(value.parents)
                .into_iter()
                .map(|p| p.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        )?)
        .with_nonce_provider(value.nonce, 0.0);
        if let Some(payload) = value.payload {
            builder = builder.with_payload(payload.try_into()?)
        }
        Ok(builder.finish()?)
    }
}
