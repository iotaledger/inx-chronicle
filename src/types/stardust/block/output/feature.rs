// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::feature as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Feature {
    #[serde(rename = "sender")]
    Sender { address: Address },
    #[serde(rename = "issuer")]
    Issuer { address: Address },
    #[serde(rename = "metadata")]
    Metadata {
        #[serde(with = "serde_bytes")]
        data: Box<[u8]>,
    },
    #[serde(rename = "tag")]
    Tag {
        #[serde(with = "serde_bytes")]
        data: Box<[u8]>,
    },
}

impl From<&bee::Feature> for Feature {
    fn from(value: &bee::Feature) -> Self {
        match value {
            bee::Feature::Sender(a) => Self::Sender {
                address: (*a.address()).into(),
            },
            bee::Feature::Issuer(a) => Self::Issuer {
                address: (*a.address()).into(),
            },
            bee::Feature::Metadata(b) => Self::Metadata {
                data: b.data().to_vec().into_boxed_slice(),
            },
            bee::Feature::Tag(b) => Self::Tag {
                data: b.tag().to_vec().into_boxed_slice(),
            },
        }
    }
}

impl TryFrom<Feature> for bee::Feature {
    type Error = crate::types::Error;

    fn try_from(value: Feature) -> Result<Self, Self::Error> {
        Ok(match value {
            Feature::Sender { address } => bee::Feature::Sender(bee::SenderFeature::new(address.into())),
            Feature::Issuer { address } => bee::Feature::Issuer(bee::IssuerFeature::new(address.into())),
            Feature::Metadata { data } => bee::Feature::Metadata(bee::MetadataFeature::new(data.into())?),
            Feature::Tag { data } => bee::Feature::Tag(bee::TagFeature::new(data.into())?),
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_feature_bson() {
        let block = get_test_sender_block(bee_test::rand::address::rand_address().into());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = get_test_issuer_block(bee_test::rand::address::rand_address().into());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = get_test_metadata_block();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = get_test_tag_block();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());
    }

    pub(crate) fn get_test_sender_block(address: Address) -> Feature {
        Feature::Sender { address }
    }

    pub(crate) fn get_test_issuer_block(address: Address) -> Feature {
        Feature::Issuer { address }
    }

    pub(crate) fn get_test_metadata_block() -> Feature {
        Feature::Metadata {
            data: "Foo".as_bytes().to_vec().into_boxed_slice(),
        }
    }

    pub(crate) fn get_test_tag_block() -> Feature {
        Feature::Tag {
            data: "Bar".as_bytes().to_vec().into_boxed_slice(),
        }
    }
}
