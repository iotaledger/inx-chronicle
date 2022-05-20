// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::payload::tagged_data as bee;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaggedDataPayload {
    #[serde(with = "serde_bytes")]
    tag: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    data: Box<[u8]>,
}

impl From<&bee::TaggedDataPayload> for TaggedDataPayload {
    fn from(value: &bee::TaggedDataPayload) -> Self {
        Self {
            tag: value.tag().to_vec().into_boxed_slice(),
            data: value.data().to_vec().into_boxed_slice(),
        }
    }
}

impl TryFrom<TaggedDataPayload> for bee::TaggedDataPayload {
    type Error = crate::types::error::Error;

    fn try_from(value: TaggedDataPayload) -> Result<Self, Self::Error> {
        Ok(bee::TaggedDataPayload::new(value.tag.into(), value.data.into())?)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_tagged_data_payload_bson() {
        let payload = get_test_tagged_data_payload();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<TaggedDataPayload>(bson).unwrap());
    }

    pub(crate) fn get_test_tagged_data_payload() -> TaggedDataPayload {
        (&bee_test::rand::payload::rand_tagged_data_payload()).into()
    }
}
