// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output::feature as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Feature {
    Sender {
        address: Address,
    },
    Issuer {
        address: Address,
    },
    Metadata {
        #[serde(with = "serde_bytes")]
        data: Box<[u8]>,
    },
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
    type Error = bee_block_stardust::Error;

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
mod test {
    use bee_block_stardust::rand::output::feature::{
        rand_issuer_feature, rand_metadata_feature, rand_sender_feature, rand_tag_feature,
    };
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_feature_bson() {
        let block = Feature::from(&bee::Feature::from(rand_sender_feature()));
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = Feature::from(&bee::Feature::from(rand_issuer_feature()));
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = Feature::from(&bee::Feature::from(rand_metadata_feature()));
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = Feature::from(&bee::Feature::from(rand_tag_feature()));
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());
    }
}
