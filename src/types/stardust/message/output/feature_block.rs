// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output::feature_block as stardust;
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

impl From<&stardust::FeatureBlock> for FeatureBlock {
    fn from(value: &stardust::FeatureBlock) -> Self {
        match value {
            stardust::FeatureBlock::Sender(a) => Self::Sender {
                address: (*a.address()).into(),
            },
            stardust::FeatureBlock::Issuer(a) => Self::Issuer {
                address: (*a.address()).into(),
            },
            stardust::FeatureBlock::Metadata(b) => Self::Metadata {
                data: b.data().to_vec().into_boxed_slice(),
            },
            stardust::FeatureBlock::Tag(b) => Self::Tag {
                data: b.tag().to_vec().into_boxed_slice(),
            },
        }
    }
}

impl TryFrom<FeatureBlock> for stardust::FeatureBlock {
    type Error = crate::types::error::Error;

    fn try_from(value: FeatureBlock) -> Result<Self, Self::Error> {
        Ok(match value {
            FeatureBlock::Sender { address } => {
                stardust::FeatureBlock::Sender(stardust::SenderFeatureBlock::new(address.try_into()?))
            }
            FeatureBlock::Issuer { address } => {
                stardust::FeatureBlock::Issuer(stardust::IssuerFeatureBlock::new(address.try_into()?))
            }
            FeatureBlock::Metadata { data } => {
                stardust::FeatureBlock::Metadata(stardust::MetadataFeatureBlock::new(data.into())?)
            }
            FeatureBlock::Tag { data } => stardust::FeatureBlock::Tag(stardust::TagFeatureBlock::new(data.into())?),
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    pub(crate) const METADATA: &str = "Foo!";
    pub(crate) const TAG: &str = "Bar!";

    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::message::address::test::{
        get_test_alias_address, get_test_ed25519_address, get_test_nft_address,
    };

    #[test]
    fn test_feature_block_bson() {
        let block = get_test_sender_block(get_test_ed25519_address());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<FeatureBlock>(bson).unwrap());

        let block = get_test_sender_block(get_test_alias_address());
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<FeatureBlock>(bson).unwrap());

        let block = get_test_issuer_block(get_test_nft_address());
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
            data: METADATA.as_bytes().to_vec().into_boxed_slice(),
        }
    }

    pub(crate) fn get_test_tag_block() -> FeatureBlock {
        FeatureBlock::Tag {
            data: TAG.as_bytes().to_vec().into_boxed_slice(),
        }
    }
}
