// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing output [`Feature`]s.

use std::borrow::Borrow;

use iota_sdk::types::block::{
    output::feature::{self as iota, Ed25519BlockIssuerKey},
    slot::{EpochIndex, SlotIndex},
};
use serde::{Deserialize, Serialize};

use crate::model::utxo::AddressDto;

/// The different feature variants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum FeatureDto {
    /// The sender feature.
    Sender {
        /// The address associated with the feature.
        address: AddressDto,
    },
    /// The issuer feature.
    Issuer {
        /// The address associated with the feature.
        address: AddressDto,
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
    /// A block issuer feature.
    BlockIssuer {
        /// The slot index at which the feature expires and can be removed.
        expiry_slot: SlotIndex,
        /// The block issuer keys.
        block_issuer_keys: Vec<Ed25519BlockIssuerKey>,
    },
    /// A staking feature.
    Staking {
        /// The amount of coins that are locked and staked in the containing account.
        staked_amount: u64,
        /// The fixed cost of the validator, which it receives as part of its Mana rewards.
        fixed_cost: u64,
        /// The epoch index in which the staking started.
        start_epoch: EpochIndex,
        /// The epoch index in which the staking ends.
        end_epoch: EpochIndex,
    },
}

impl<T: Borrow<iota::Feature>> From<T> for FeatureDto {
    fn from(value: T) -> Self {
        match value.borrow() {
            iota::Feature::Sender(a) => Self::Sender {
                address: a.address().into(),
            },
            iota::Feature::Issuer(a) => Self::Issuer {
                address: a.address().into(),
            },
            iota::Feature::Metadata(b) => Self::Metadata {
                data: b.data().to_vec().into_boxed_slice(),
            },
            iota::Feature::Tag(b) => Self::Tag {
                data: b.tag().to_vec().into_boxed_slice(),
            },
            iota::Feature::BlockIssuer(f) => Self::BlockIssuer {
                expiry_slot: f.expiry_slot(),
                block_issuer_keys: f.block_issuer_keys().iter().map(|b| *b.as_ed25519()).collect(),
            },
            iota::Feature::Staking(f) => Self::Staking {
                staked_amount: f.staked_amount(),
                fixed_cost: f.fixed_cost(),
                start_epoch: f.start_epoch(),
                end_epoch: f.end_epoch(),
            },
        }
    }
}

impl TryFrom<FeatureDto> for iota::Feature {
    type Error = iota_sdk::types::block::Error;

    fn try_from(value: FeatureDto) -> Result<Self, Self::Error> {
        Ok(match value {
            FeatureDto::Sender { address } => iota::Feature::Sender(iota::SenderFeature::new(address)),
            FeatureDto::Issuer { address } => iota::Feature::Issuer(iota::IssuerFeature::new(address)),
            FeatureDto::Metadata { data } => iota::Feature::Metadata(iota::MetadataFeature::new(data)?),
            FeatureDto::Tag { data } => iota::Feature::Tag(iota::TagFeature::new(data)?),
            FeatureDto::BlockIssuer {
                expiry_slot,
                block_issuer_keys,
            } => iota::Feature::BlockIssuer(iota::BlockIssuerFeature::new(
                expiry_slot,
                block_issuer_keys.into_iter().map(|b| iota::BlockIssuerKey::Ed25519(b)),
            )?),
            FeatureDto::Staking {
                staked_amount,
                fixed_cost,
                start_epoch,
                end_epoch,
            } => iota::Feature::Staking(iota::StakingFeature::new(
                staked_amount,
                fixed_cost,
                start_epoch,
                end_epoch,
            )),
        })
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_feature_bson() {
//         let block = FeatureDto::rand_sender();
//         iota::Feature::try_from(block.clone()).unwrap();
//         let bson = to_bson(&block).unwrap();
//         assert_eq!(block, from_bson::<FeatureDto>(bson).unwrap());

//         let block = FeatureDto::rand_issuer();
//         iota::Feature::try_from(block.clone()).unwrap();
//         let bson = to_bson(&block).unwrap();
//         assert_eq!(block, from_bson::<FeatureDto>(bson).unwrap());

//         let block = FeatureDto::rand_metadata();
//         iota::Feature::try_from(block.clone()).unwrap();
//         let bson = to_bson(&block).unwrap();
//         assert_eq!(block, from_bson::<FeatureDto>(bson).unwrap());

//         let block = FeatureDto::rand_tag();
//         iota::Feature::try_from(block.clone()).unwrap();
//         let bson = to_bson(&block).unwrap();
//         assert_eq!(block, from_bson::<FeatureDto>(bson).unwrap());
//     }
// }
