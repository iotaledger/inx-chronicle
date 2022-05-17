// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output::feature_block as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::message::Address;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FeatureBlock {
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

impl From<&bee::FeatureBlock> for FeatureBlock {
    fn from(value: &bee::FeatureBlock) -> Self {
        match value {
            bee::FeatureBlock::Sender(a) => Self::Sender {
                address: (*a.address()).into(),
            },
            bee::FeatureBlock::Issuer(a) => Self::Issuer {
                address: (*a.address()).into(),
            },
            bee::FeatureBlock::Metadata(b) => Self::Metadata {
                data: b.data().to_vec().into_boxed_slice(),
            },
            bee::FeatureBlock::Tag(b) => Self::Tag {
                data: b.tag().to_vec().into_boxed_slice(),
            },
        }
    }
}

impl TryFrom<FeatureBlock> for bee::FeatureBlock {
    type Error = crate::types::error::Error;

    fn try_from(value: FeatureBlock) -> Result<Self, Self::Error> {
        Ok(match value {
            FeatureBlock::Sender { address } => {
                bee::FeatureBlock::Sender(bee::SenderFeatureBlock::new(address.try_into()?))
            }
            FeatureBlock::Issuer { address } => {
                bee::FeatureBlock::Issuer(bee::IssuerFeatureBlock::new(address.try_into()?))
            }
            FeatureBlock::Metadata { data } => {
                bee::FeatureBlock::Metadata(bee::MetadataFeatureBlock::new(data.into())?)
            }
            FeatureBlock::Tag { data } => bee::FeatureBlock::Tag(bee::TagFeatureBlock::new(data.into())?),
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_feature_block_bson() {
        let block = get_test_sender_block(bee_test::rand::address::rand_address().into());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<FeatureBlock>(bson).unwrap());

        let block = get_test_issuer_block(bee_test::rand::address::rand_address().into());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<FeatureBlock>(bson).unwrap());

        let block = get_test_metadata_block();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<FeatureBlock>(bson).unwrap());

        let block = get_test_tag_block();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<FeatureBlock>(bson).unwrap());
    }

    pub(crate) fn get_test_sender_block(address: Address) -> FeatureBlock {
        FeatureBlock::Sender { address }
    }

    pub(crate) fn get_test_issuer_block(address: Address) -> FeatureBlock {
        FeatureBlock::Issuer { address }
    }

    pub(crate) fn get_test_metadata_block() -> FeatureBlock {
        FeatureBlock::Metadata {
            data: "Foo".as_bytes().to_vec().into_boxed_slice(),
        }
    }

    pub(crate) fn get_test_tag_block() -> FeatureBlock {
        FeatureBlock::Tag {
            data: "Bar".as_bytes().to_vec().into_boxed_slice(),
        }
    }
}
