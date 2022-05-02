// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust as stardust;
use serde::{Deserialize, Serialize};

use super::payload::Payload;

pub type MessageId = Box<[u8]>;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub protocol_version: u8,
    pub parents: Box<[MessageId]>,
    pub payload: Option<Payload>,
    pub nonce: u64,
}

impl From<stardust::Message> for Message {
    fn from(value: stardust::Message) -> Self {
        Self {
            protocol_version: value.protocol_version().into(),
            parents: todo!(),
            payload: todo!(),
            nonce: value.nonce().into(),
        }
    }
}
