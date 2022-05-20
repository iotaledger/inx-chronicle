// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_message_stardust as bee;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Hash, Ord, PartialOrd, Eq)]
#[serde(transparent)]
pub struct MessageId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl MessageId {
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<bee::MessageId> for MessageId {
    fn from(value: bee::MessageId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<MessageId> for bee::MessageId {
    type Error = crate::types::error::Error;

    fn try_from(value: MessageId) -> Result<Self, Self::Error> {
        Ok(bee::MessageId::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for MessageId {
    type Err = crate::types::error::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::MessageId::from_str(s)?.into())
    }
}
