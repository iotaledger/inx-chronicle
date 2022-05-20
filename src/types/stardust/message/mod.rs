// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod address;
mod input;
mod message_id;
mod output;
mod payload;
mod signature;
mod unlock_block;

use bee_message_stardust as bee;
use serde::{Deserialize, Serialize};

pub use self::{address::*, input::*, message_id::*, output::*, payload::*, signature::*, unlock_block::*};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "_id")]
    pub message_id: MessageId,
    pub protocol_version: u8,
    pub parents: Box<[MessageId]>,
    pub payload: Option<Payload>,
    #[serde(with = "crate::types::stringify")]
    pub nonce: u64,
}

impl From<bee::Message> for Message {
    fn from(value: bee::Message) -> Self {
        Self {
            message_id: value.id().into(),
            protocol_version: value.protocol_version(),
            parents: value.parents().iter().map(|id| MessageId::from(*id)).collect(),
            payload: value.payload().map(Into::into),
            nonce: value.nonce(),
        }
    }
}

impl TryFrom<Message> for bee::Message {
    type Error = crate::types::error::Error;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        let mut builder = bee::MessageBuilder::<u64>::new(bee::parent::Parents::new(
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

#[cfg(test)]
mod tests {
    use mongodb::bson::{from_bson, to_bson};

    use super::{
        payload::test::{get_test_milestone_payload, get_test_tagged_data_payload, get_test_transaction_payload},
        *,
    };

    #[test]
    fn test_message_id_bson() {
        let message_id = MessageId::from(bee_test::rand::message::rand_message_id());
        let bson = to_bson(&message_id).unwrap();
        from_bson::<MessageId>(bson).unwrap();
    }

    #[test]
    fn test_message_bson() {
        let message = get_test_transaction_message();
        let bson = to_bson(&message).unwrap();
        assert_eq!(message, from_bson::<Message>(bson).unwrap());

        let message = get_test_milestone_message();
        let bson = to_bson(&message).unwrap();
        assert_eq!(message, from_bson::<Message>(bson).unwrap());

        let message = get_test_tagged_data_message();
        let bson = to_bson(&message).unwrap();
        assert_eq!(message, from_bson::<Message>(bson).unwrap());
    }

    fn get_test_transaction_message() -> Message {
        Message::from(
            bee::MessageBuilder::<u64>::new(bee_test::rand::parents::rand_parents())
                .with_nonce_provider(u64::MAX, 0.0)
                .with_payload(get_test_transaction_payload().try_into().unwrap())
                .finish()
                .unwrap(),
        )
    }

    fn get_test_milestone_message() -> Message {
        Message::from(
            bee::MessageBuilder::<u64>::new(bee_test::rand::parents::rand_parents())
                .with_nonce_provider(u64::MAX, 0.0)
                .with_payload(get_test_milestone_payload().try_into().unwrap())
                .finish()
                .unwrap(),
        )
    }

    fn get_test_tagged_data_message() -> Message {
        Message::from(
            bee::MessageBuilder::<u64>::new(bee_test::rand::parents::rand_parents())
                .with_nonce_provider(u64::MAX, 0.0)
                .with_payload(get_test_tagged_data_payload().try_into().unwrap())
                .finish()
                .unwrap(),
        )
    }
}
