// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use iota_types::block::output::feature as iota;
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

impl<T: Borrow<iota::Feature>> From<T> for Feature {
    fn from(value: T) -> Self {
        match value.borrow() {
            iota::Feature::Sender(a) => Self::Sender {
                address: (*a.address()).into(),
            },
            iota::Feature::Issuer(a) => Self::Issuer {
                address: (*a.address()).into(),
            },
            iota::Feature::Metadata(b) => Self::Metadata {
                data: b.data().to_vec().into_boxed_slice(),
            },
            iota::Feature::Tag(b) => Self::Tag {
                data: b.tag().to_vec().into_boxed_slice(),
            },
        }
    }
}

impl TryFrom<Feature> for iota::Feature {
    type Error = iota_types::block::Error;

    fn try_from(value: Feature) -> Result<Self, Self::Error> {
        Ok(match value {
            Feature::Sender { address } => iota::Feature::Sender(iota::SenderFeature::new(address.into())),
            Feature::Issuer { address } => iota::Feature::Issuer(iota::IssuerFeature::new(address.into())),
            Feature::Metadata { data } => iota::Feature::Metadata(iota::MetadataFeature::new(data.into())?),
            Feature::Tag { data } => iota::Feature::Tag(iota::TagFeature::new(data.into())?),
        })
    }
}

impl From<Feature> for iota::dto::FeatureDto {
    fn from(value: Feature) -> Self {
        match value {
            Feature::Sender { address } => Self::Sender(iota::dto::SenderFeatureDto {
                kind: iota::SenderFeature::KIND,
                address: address.into(),
            }),
            Feature::Issuer { address } => Self::Issuer(iota::dto::IssuerFeatureDto {
                kind: iota::IssuerFeature::KIND,
                address: address.into(),
            }),
            Feature::Metadata { data } => Self::Metadata(iota::dto::MetadataFeatureDto {
                kind: iota::MetadataFeature::KIND,
                data: prefix_hex::encode(data),
            }),
            Feature::Tag { data } => Self::Tag(iota::dto::TagFeatureDto {
                kind: iota::TagFeature::KIND,
                tag: prefix_hex::encode(data),
            }),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::{
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
            iota::Feature::from(rand_sender_feature()).into()
        }

        /// Generates a random issuer [`Feature`].
        pub fn rand_issuer() -> Self {
            iota::Feature::from(rand_issuer_feature()).into()
        }

        /// Generates a random metadata [`Feature`].
        pub fn rand_metadata() -> Self {
            iota::Feature::from(rand_metadata_feature()).into()
        }

        /// Generates a random tag [`Feature`].
        pub fn rand_tag() -> Self {
            iota::Feature::from(rand_tag_feature()).into()
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
        iota::Feature::try_from(block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = Feature::rand_issuer();
        iota::Feature::try_from(block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = Feature::rand_metadata();
        iota::Feature::try_from(block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());

        let block = Feature::rand_tag();
        iota::Feature::try_from(block.clone()).unwrap();
        let bson = to_bson(&block).unwrap();
        assert_eq!(block, from_bson::<Feature>(bson).unwrap());
    }
}
