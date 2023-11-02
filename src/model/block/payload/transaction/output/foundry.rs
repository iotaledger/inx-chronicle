// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the foundry output.

use std::borrow::Borrow;

use iota_sdk::{
    types::block::{
        address::Address,
        output::{self as iota, FoundryId},
    },
    utils::serde::string,
};
use serde::{Deserialize, Serialize};

use super::{unlock_condition::ImmutableAccountAddressUnlockConditionDto, FeatureDto, NativeTokenDto, TokenSchemeDto};

/// Represents a foundry in the UTXO model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryOutputDto {
    /// The output amount.
    #[serde(with = "string")]
    pub amount: u64,
    /// The list of [`NativeToken`]s.
    pub native_tokens: Vec<NativeTokenDto>,
    /// The associated id of the foundry.
    pub foundry_id: FoundryId,
    /// The serial number of the foundry.
    #[serde(with = "string")]
    pub serial_number: u32,
    /// The [`TokenScheme`] of the underlying token.
    pub token_scheme: TokenSchemeDto,
    /// The immutable alias address unlock condition.
    pub immutable_account_address_unlock_condition: ImmutableAccountAddressUnlockConditionDto,
    /// The corresponding list of [`Feature`]s.
    pub features: Vec<FeatureDto>,
    /// The corresponding list of immutable [`Feature`]s.
    pub immutable_features: Vec<FeatureDto>,
}

impl FoundryOutputDto {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "foundry";
}

impl<T: Borrow<iota::FoundryOutput>> From<T> for FoundryOutputDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount().into(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            foundry_id: value.id().into(),
            serial_number: value.serial_number(),
            token_scheme: value.token_scheme().into(),
            immutable_account_address_unlock_condition: ImmutableAccountAddressUnlockConditionDto {
                address: Address::from(*value.account_address()).into(),
            },
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
//     fn test_foundry_output_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let output = FoundryOutputDto::rand(&ctx);
//         iota::FoundryOutput::try_from(output.clone()).unwrap();
//         let bson = to_bson(&output).unwrap();
//         assert_eq!(output, from_bson::<FoundryOutputDto>(bson).unwrap());
//     }
// }
