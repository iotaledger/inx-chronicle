// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{borrow::Borrow, str::FromStr};

use bee_block_stardust::output as bee;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use super::{
    unlock_condition::{
        AddressUnlockCondition, ExpirationUnlockCondition, StorageDepositReturnUnlockCondition, TimelockUnlockCondition,
    },
    Feature, NativeToken, OutputAmount,
};
use crate::types::util::bytify;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NftId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl NftId {
    const LENGTH: usize = bee::NftId::LENGTH;

    pub fn from_output_id_str(s: &str) -> Result<Self, bee_block_stardust::Error> {
        Ok(bee::NftId::from(bee::OutputId::from_str(s)?).into())
    }
}

impl From<bee::NftId> for NftId {
    fn from(value: bee::NftId) -> Self {
        Self(*value)
    }
}

impl From<NftId> for bee::NftId {
    fn from(value: NftId) -> Self {
        bee::NftId::new(value.0)
    }
}

impl FromStr for NftId {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::NftId::from_str(s)?.into())
    }
}

impl From<NftId> for Bson {
    fn from(val: NftId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NftOutput {
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub nft_id: NftId,
    pub address_unlock_condition: AddressUnlockCondition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_deposit_return_unlock_condition: Option<StorageDepositReturnUnlockCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timelock_unlock_condition: Option<TimelockUnlockCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_unlock_condition: Option<ExpirationUnlockCondition>,
    pub features: Box<[Feature]>,
    pub immutable_features: Box<[Feature]>,
}

impl<T: Borrow<bee::NftOutput>> From<T> for NftOutput {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount().into(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            nft_id: (*value.nft_id()).into(),
            // Panic: The address unlock condition has to be present.
            address_unlock_condition: value.unlock_conditions().address().unwrap().into(),
            storage_deposit_return_unlock_condition: value.unlock_conditions().storage_deposit_return().map(Into::into),
            timelock_unlock_condition: value.unlock_conditions().timelock().map(Into::into),
            expiration_unlock_condition: value.unlock_conditions().expiration().map(Into::into),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<NftOutput> for bee::NftOutput {
    type Error = bee_block_stardust::Error;

    fn try_from(value: NftOutput) -> Result<Self, Self::Error> {
        // The order of the conditions is imporant here because unlock conditions have to be sorted by type.
        let unlock_conditions = [
            Some(bee::unlock_condition::AddressUnlockCondition::from(value.address_unlock_condition).into()),
            value
                .storage_deposit_return_unlock_condition
                .map(bee::unlock_condition::StorageDepositReturnUnlockCondition::try_from)
                .transpose()?
                .map(Into::into),
            value
                .timelock_unlock_condition
                .map(bee::unlock_condition::TimelockUnlockCondition::try_from)
                .transpose()?
                .map(Into::into),
            value
                .expiration_unlock_condition
                .map(bee::unlock_condition::ExpirationUnlockCondition::try_from)
                .transpose()?
                .map(Into::into),
        ];

        Self::build_with_amount(value.amount.0, value.nft_id.into())?
            .with_native_tokens(
                Vec::from(value.native_tokens)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_unlock_conditions(unlock_conditions.into_iter().flatten())
            .with_features(
                Vec::from(value.features)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_immutable_features(
                Vec::from(value.immutable_features)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .finish()
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::{bytes::rand_bytes_array, output::rand_nft_output};

    use super::*;

    impl NftId {
        /// Generates a random [`NftId`].
        pub fn rand() -> Self {
            Self(rand_bytes_array())
        }
    }

    impl NftOutput {
        /// Generates a random [`NftOutput`].
        pub fn rand() -> Self {
            rand_nft_output().into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_nft_id_bson() {
        let nft_id = NftId::rand();
        let bson = to_bson(&nft_id).unwrap();
        assert_eq!(Bson::from(nft_id), bson);
        assert_eq!(nft_id, from_bson::<NftId>(bson).unwrap());
    }

    #[test]
    fn test_nft_output_bson() {
        let output = NftOutput::rand();
        bee::NftOutput::try_from(output.clone()).unwrap();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<NftOutput>(bson).unwrap());
    }
}
