// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::{
    output::{
        feature::{FeatureFlags, MetadataFeature},
        Feature,
    },
    rand::{
        bytes::rand_bytes,
        number::rand_number_range,
        output::feature::{rand_issuer_feature, rand_sender_feature, rand_tag_feature},
    },
};

/// Generates a random [`MetadataFeature`].
pub fn rand_metadata_feature(max_bytes: impl Into<Option<u16>>) -> MetadataFeature {
    let bytes = rand_bytes(rand_number_range(
        1..=max_bytes.into().unwrap_or(*MetadataFeature::LENGTH_RANGE.end()).max(1),
    ) as usize);
    MetadataFeature::new(bytes).unwrap()
}

fn rand_feature_from_flag(flag: &FeatureFlags) -> Feature {
    match *flag {
        FeatureFlags::SENDER => Feature::Sender(rand_sender_feature()),
        FeatureFlags::ISSUER => Feature::Issuer(rand_issuer_feature()),
        FeatureFlags::METADATA => Feature::Metadata(rand_metadata_feature(100)),
        FeatureFlags::TAG => Feature::Tag(rand_tag_feature()),
        _ => unreachable!(),
    }
}

/// Generates a [`Vec`] of random [`Feature`]s given a set of allowed [`FeatureFlags`].
pub fn rand_allowed_features(allowed_features: FeatureFlags) -> Vec<Feature> {
    let mut all_features = FeatureFlags::ALL_FLAGS
        .iter()
        .map(rand_feature_from_flag)
        .collect::<Vec<_>>();
    all_features.retain(|feature| allowed_features.contains(feature.flag()));
    all_features
}
