// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::payload::tagged_data as bee;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaggedDataPayload {
    #[serde(with = "serde_bytes")]
    tag: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    data: Box<[u8]>,
}

impl<T: Borrow<bee::TaggedDataPayload>> From<T> for TaggedDataPayload {
    fn from(value: T) -> Self {
        Self {
            tag: value.borrow().tag().to_vec().into_boxed_slice(),
            data: value.borrow().data().to_vec().into_boxed_slice(),
        }
    }
}

impl TryFrom<TaggedDataPayload> for bee::TaggedDataPayload {
    type Error = bee_block_stardust::Error;

    fn try_from(value: TaggedDataPayload) -> Result<Self, Self::Error> {
        bee::TaggedDataPayload::new(value.tag.into(), value.data.into())
    }
}

impl From<TaggedDataPayload> for bee::dto::TaggedDataPayloadDto {
    fn from(value: TaggedDataPayload) -> Self {
        Self {
            kind: bee::TaggedDataPayload::KIND,
            tag: prefix_hex::encode(value.tag),
            data: prefix_hex::encode(value.data),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::payload::rand_tagged_data_payload;

    use super::*;

    impl TaggedDataPayload {
        /// Generates a random [`TaggedDataPayload`].
        pub fn rand() -> Self {
            rand_tagged_data_payload().into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_tagged_data_payload_bson() {
        let payload = TaggedDataPayload::rand();
        bee::TaggedDataPayload::try_from(payload.clone()).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<TaggedDataPayload>(bson).unwrap());
    }
}
