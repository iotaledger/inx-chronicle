// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::output::feature as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

/// The different [`Feature`] variants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Feature {
    /// The sender feature.
    Sender {
        /// The address associated with the feature.
        address: Address,
    },
    /// The issuer feature.
    Issuer {
        /// The address associated with the feature.
        address: Address,
    },
    /// The metadata feature.
    Metadata {
        /// The data of the feature.
        #[serde(with = "serde_bytes")]
        data: Box<[u8]>,
    },
    /// The tag feature.
    Tag {
        /// The data of the feature.
        #[serde(with = "serde_bytes")]
        data: Box<[u8]>,
    },
}

impl<T: Borrow<bee::Feature>> From<T> for Feature {
    fn from(value: T) -> Self {
        match value.borrow() {
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

impl From<Feature> for bee::dto::FeatureDto {
    fn from(value: Feature) -> Self {
        match value {
            Feature::Sender { address } => Self::Sender(bee::dto::SenderFeatureDto {
                kind: bee::SenderFeature::KIND,
                address: address.into(),
            }),
            Feature::Issuer { address } => Self::Issuer(bee::dto::IssuerFeatureDto {
                kind: bee::IssuerFeature::KIND,
                address: address.into(),
            }),
            Feature::Metadata { data } => Self::Metadata(bee::dto::MetadataFeatureDto {
                kind: bee::MetadataFeature::KIND,
                data: prefix_hex::encode(data),
            }),
            Feature::Tag { data } => Self::Tag(bee::dto::TagFeatureDto {
                kind: bee::TagFeature::KIND,
                tag: prefix_hex::encode(data),
            }),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::{
        output::feature::FeatureFlags,
        rand::output::feature::{
            rand_allowed_features, rand_issuer_feature, rand_metadata_feature, rand_sender_feature, rand_tag_feature,
        },
    };

    use super::*;

    impl Feature {
        /// Generates a random [`Feature`].
        pub fn rand_allowed_features(allowed_features: FeatureFlags) -> Vec<Self> {
            rand_allowed_features(allowed_features)
                .into_iter()
                .map(Into::into)
                .collect()
        }

        /// Generates a random sender [`Feature`].
        pub fn rand_sender() -> Self {
            bee::Feature::from(rand_sender_feature()).into()
        }

        /// Generates a random issuer [`Feature`].
        pub fn rand_issuer() -> Self {
            bee::Feature::from(rand_issuer_feature()).into()
        }

        /// Generates a random metadata [`Feature`].
        pub fn rand_metadata() -> Self {
            bee::Feature::from(rand_metadata_feature()).into()
        }

        /// Generates a random tag [`Feature`].
        pub fn rand_tag() -> Self {
            bee::Feature::from(rand_tag_feature()).into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_feature_bson() {
        let block = Feature::rand_sender();
        bee::Feature::try_from(block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = Feature::rand_issuer();
        bee::Feature::try_from(block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = Feature::rand_metadata();
        bee::Feature::try_from(block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = Feature::rand_tag();
        bee::Feature::try_from(block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());
    }
}
