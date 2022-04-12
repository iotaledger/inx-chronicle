// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::MessageId;
use packable::PackableExt;
use serde::{Deserialize, Serialize};

use crate::{db::Model, inx::InxError};

#[derive(Debug, Serialize, Deserialize)]
/// A record that stores information about a message.
pub struct Message {
    message_id: MessageId,
    raw_bytes: Vec<u8>,
}

impl Model for Message {
    const COLLECTION: &'static str = "stardust_messages";
}

impl TryFrom<inx::proto::Message> for Message {
    type Error = InxError;

    fn try_from(message: inx::proto::Message) -> Result<Self, Self::Error> {
        let mut message_id_bytes = message.message_id.ok_or(InxError::MissingField("message_id"))?.id;
        let message_id =
            MessageId::unpack_verified(&mut message_id_bytes).map_err(|_| InxError::InvalidField("message_id"))?;
        let raw_bytes = message.message.ok_or(InxError::MissingField("message"))?.data;
        Ok(Message { message_id, raw_bytes })
    }
}
