// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`AccountOutput`].

use std::borrow::Borrow;

use iota_sdk::{
    types::block::output::{self as iota, AccountId},
    utils::serde::string,
};
use serde::{Deserialize, Serialize};

use super::{feature::FeatureDto, native_token::NativeTokenDto, unlock_condition::AddressUnlockConditionDto};

/// Describes an account in the ledger that can be controlled by the state and governance controllers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountOutputDto {
    /// Amount of IOTA coins held by the output.
    #[serde(with = "string")]
    pub amount: u64,
    /// Amount of mana held by the output.
    #[serde(with = "string")]
    pub mana: u64,
    /// Native tokens held by the output.
    pub native_tokens: Vec<NativeTokenDto>,
    /// Unique identifier of the account.
    pub account_id: AccountId,
    /// A counter that denotes the number of foundries created by this account.
    pub foundry_counter: u32,
    /// The address unlock condition.
    pub address_unlock_condition: AddressUnlockConditionDto,
    pub features: Vec<FeatureDto>,
    pub immutable_features: Vec<FeatureDto>,
}

impl AccountOutputDto {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "account";
}

impl<T: Borrow<iota::AccountOutput>> From<T> for AccountOutputDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount().into(),
            mana: value.mana(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            account_id: *value.account_id(),
            foundry_counter: value.foundry_counter(),
            address_unlock_condition: AddressUnlockConditionDto {
                address: value.address().into(),
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
//     fn test_alias_id_bson() {
//         let alias_id = AliasId::rand();
//         let bson = to_bson(&alias_id).unwrap();
//         assert_eq!(Bson::from(alias_id), bson);
//         assert_eq!(alias_id, from_bson::<AliasId>(bson).unwrap());
//     }

//     #[test]
//     fn test_alias_output_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let output = AccountOutputDto::rand(&ctx);
//         iota::AliasOutput::try_from(output.clone()).unwrap();
//         let bson = to_bson(&output).unwrap();
//         assert_eq!(output, from_bson::<AccountOutputDto>(bson).unwrap());
//     }
// }
