// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`TaggedDataPayload`] type.

use std::borrow::Borrow;

use iota_sdk::types::block::payload::tagged_data as iota;
use serde::{Deserialize, Serialize};

/// Represents the tagged data payload for data blocks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaggedDataPayloadDto {
    #[serde(with = "serde_bytes")]
    tag: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    data: Box<[u8]>,
}

impl TaggedDataPayloadDto {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "tagged_data";
}

impl<T: Borrow<iota::TaggedDataPayload>> From<T> for TaggedDataPayloadDto {
    fn from(value: T) -> Self {
        Self {
            tag: value.borrow().tag().to_vec().into_boxed_slice(),
            data: value.borrow().data().to_vec().into_boxed_slice(),
        }
    }
}

impl TryFrom<TaggedDataPayloadDto> for iota::TaggedDataPayload {
    type Error = iota_sdk::types::block::Error;

    fn try_from(value: TaggedDataPayloadDto) -> Result<Self, Self::Error> {
        iota::TaggedDataPayload::new(value.tag, value.data)
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_tagged_data_payload_bson() {
//         let payload = TaggedDataPayloadDto::rand();
//         iota::TaggedDataPayload::try_from(payload.clone()).unwrap();
//         let bson = to_bson(&payload).unwrap();
//         assert_eq!(payload, from_bson::<TaggedDataPayloadDto>(bson).unwrap());
//     }
// }
