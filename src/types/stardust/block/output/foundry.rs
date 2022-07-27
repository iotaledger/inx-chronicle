// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::output as bee;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use super::{unlock_condition::ImmutableAliasAddressUnlockCondition, Feature, NativeToken, OutputAmount, TokenScheme};
use crate::types::util::bytify;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FoundryId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl FoundryId {
    const LENGTH: usize = bee::FoundryId::LENGTH;
}

impl From<bee::FoundryId> for FoundryId {
    fn from(value: bee::FoundryId) -> Self {
        Self(*value)
    }
}

impl From<FoundryId> for bee::FoundryId {
    fn from(value: FoundryId) -> Self {
        bee::FoundryId::new(value.0)
    }
}

impl FromStr for FoundryId {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::FoundryId::from_str(s)?.into())
    }
}

impl From<FoundryId> for Bson {
    fn from(val: FoundryId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoundryOutput {
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub foundry_id: FoundryId,
    #[serde(with = "crate::types::util::stringify")]
    pub serial_number: u32,
    pub token_scheme: TokenScheme,
    pub immutable_alias_address_unlock_condition: ImmutableAliasAddressUnlockCondition,
    pub features: Box<[Feature]>,
    pub immutable_features: Box<[Feature]>,
}

impl From<&bee::FoundryOutput> for FoundryOutput {
    fn from(value: &bee::FoundryOutput) -> Self {
        Self {
            amount: value.amount().into(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            foundry_id: value.id().into(),
            serial_number: value.serial_number(),
            token_scheme: value.token_scheme().into(),
            // Panic: The immutable alias address unlock condition has to be present.
            immutable_alias_address_unlock_condition: value
                .unlock_conditions()
                .immutable_alias_address()
                .unwrap()
                .into(),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<FoundryOutput> for bee::FoundryOutput {
    type Error = bee_block_stardust::Error;

    fn try_from(value: FoundryOutput) -> Result<Self, Self::Error> {
        let u: bee::UnlockCondition = bee::unlock_condition::ImmutableAliasAddressUnlockCondition::try_from(
            value.immutable_alias_address_unlock_condition,
        )?
        .into();

        Self::build_with_amount(value.amount.0, value.serial_number, value.token_scheme.try_into()?)?
            .with_native_tokens(
                Vec::from(value.native_tokens)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_unlock_conditions([u])
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

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::block::output::{
        feature::test::get_test_metadata_block, native_token::test::get_test_native_token,
        unlock_condition::test::rand_immutable_alias_address_unlock_condition,
    };

    #[test]
    fn test_foundry_output_bson() {
        let output = get_test_foundry_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<FoundryOutput>(bson).unwrap());
    }

    pub(crate) fn get_test_foundry_output() -> FoundryOutput {
        FoundryOutput::from(
            &bee::FoundryOutput::build_with_amount(
                100,
                bee_block_stardust::rand::number::rand_number(),
                bee::TokenScheme::Simple(bee::SimpleTokenScheme::new(250.into(), 200.into(), 300.into()).unwrap()),
            )
            .unwrap()
            .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
            .with_unlock_conditions([rand_immutable_alias_address_unlock_condition().into()])
            .with_features(vec![get_test_metadata_block().try_into().unwrap()])
            .with_immutable_features(vec![get_test_metadata_block().try_into().unwrap()])
            .finish()
            .unwrap(),
        )
    }
}
