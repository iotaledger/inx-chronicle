// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the nft output.

use std::borrow::Borrow;

use iota_sdk::types::block::output::{self as iota, NftId};
use serde::{Deserialize, Serialize};

use super::{
    unlock_condition::{
        AddressUnlockConditionDto, ExpirationUnlockConditionDto, StorageDepositReturnUnlockConditionDto,
        TimelockUnlockConditionDto,
    },
    FeatureDto, NativeTokenDto,
};

/// Represents an NFT in the UTXO model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NftOutputDto {
    // Amount of IOTA coins held by the output.
    pub amount: u64,
    // Amount of mana held by the output.
    pub mana: u64,
    /// Native tokens held by the output.
    pub native_tokens: Vec<NativeTokenDto>,
    /// The associated id of the NFT.
    pub nft_id: NftId,
    /// The address unlock condition.
    pub address_unlock_condition: AddressUnlockConditionDto,
    /// The storage deposit return unlock condition (SDRUC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_deposit_return_unlock_condition: Option<StorageDepositReturnUnlockConditionDto>,
    /// The timelock unlock condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timelock_unlock_condition: Option<TimelockUnlockConditionDto>,
    /// The expiration unlock condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_unlock_condition: Option<ExpirationUnlockConditionDto>,
    /// The corresponding list of [`Feature`]s.
    pub features: Vec<FeatureDto>,
    /// The corresponding list of immutable [`Feature`]s.
    pub immutable_features: Vec<FeatureDto>,
}

impl NftOutputDto {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "nft";
}

impl<T: Borrow<iota::NftOutput>> From<T> for NftOutputDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount(),
            mana: value.mana(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            nft_id: (*value.nft_id()).into(),
            address_unlock_condition: AddressUnlockConditionDto {
                address: value.address().into(),
            },
            storage_deposit_return_unlock_condition: value.unlock_conditions().storage_deposit_return().map(Into::into),
            timelock_unlock_condition: value.unlock_conditions().timelock().map(Into::into),
            expiration_unlock_condition: value.unlock_conditions().expiration().map(Into::into),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_nft_id_bson() {
//         let nft_id = NftId::rand();
//         let bson = to_bson(&nft_id).unwrap();
//         assert_eq!(Bson::from(nft_id), bson);
//         assert_eq!(nft_id, from_bson::<NftId>(bson).unwrap());
//     }

//     #[test]
//     fn test_nft_output_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let output = NftOutput::rand(&ctx);
//         iota::NftOutput::try_from_with_context(&ctx, output.clone()).unwrap();
//         let bson = to_bson(&output).unwrap();
//         assert_eq!(output, from_bson::<NftOutput>(bson).unwrap());
//     }
// }
