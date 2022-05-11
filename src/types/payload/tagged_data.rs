// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::payload::tagged_data as stardust;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaggedDataPayload {
    #[serde(with = "serde_bytes")]
    tag: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    data: Box<[u8]>,
}

impl From<&stardust::TaggedDataPayload> for TaggedDataPayload {
    fn from(value: &stardust::TaggedDataPayload) -> Self {
        Self {
            tag: value.tag().to_vec().into_boxed_slice(),
            data: value.data().to_vec().into_boxed_slice(),
        }
    }
}

impl TryFrom<TaggedDataPayload> for stardust::TaggedDataPayload {
    type Error = crate::dto::error::Error;

    fn try_from(value: TaggedDataPayload) -> Result<Self, Self::Error> {
        Ok(stardust::TaggedDataPayload::new(value.tag.into(), value.data.into())?)
    }
}
