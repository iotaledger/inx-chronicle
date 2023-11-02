// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`BasicOutput`].

use std::borrow::Borrow;

use iota_sdk::types::block::output as iota;
use serde::{Deserialize, Serialize};

use super::{
    unlock_condition::{
        AddressUnlockConditionDto, ExpirationUnlockConditionDto, StorageDepositReturnUnlockConditionDto,
        TimelockUnlockConditionDto,
    },
    FeatureDto, NativeTokenDto,
};

/// Represents a basic output in the UTXO model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicOutputDto {
    // Amount of IOTA coins held by the output.
    pub amount: u64,
    // Amount of mana held by the output.
    pub mana: u64,
    /// Native tokens held by the output.
    pub native_tokens: Vec<NativeTokenDto>,
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
}

impl BasicOutputDto {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "basic";
}

impl<T: Borrow<iota::BasicOutput>> From<T> for BasicOutputDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount(),
            mana: value.mana(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            address_unlock_condition: AddressUnlockConditionDto {
                address: value.address().into(),
            },
            storage_deposit_return_unlock_condition: value.unlock_conditions().storage_deposit_return().map(Into::into),
            timelock_unlock_condition: value.unlock_conditions().timelock().map(Into::into),
            expiration_unlock_condition: value.unlock_conditions().expiration().map(Into::into),
            features: value.features().iter().map(Into::into).collect(),
        }
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_basic_output_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let output = BasicOutputDto::rand(&ctx);
//         iota::BasicOutput::try_from_with_context(&ctx, output.clone()).unwrap();
//         let bson = to_bson(&output).unwrap();
//         assert_eq!(output, from_bson::<BasicOutputDto>(bson).unwrap());
//     }
// }
