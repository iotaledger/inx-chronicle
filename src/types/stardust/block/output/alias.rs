// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::output as bee;
use serde::{Deserialize, Serialize};

use super::{feature::Feature, native_token::NativeToken, unlock_condition::UnlockCondition, OutputAmount};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AliasId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl AliasId {
    pub fn from_output_id_str(s: &str) -> Result<Self, crate::types::error::Error> {
        Ok(bee::AliasId::from(bee::OutputId::from_str(s)?).into())
    }
}

impl From<bee::AliasId> for AliasId {
    fn from(value: bee::AliasId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<AliasId> for bee::AliasId {
    type Error = crate::types::error::Error;

    fn try_from(value: AliasId) -> Result<Self, Self::Error> {
        Ok(bee::AliasId::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for AliasId {
    type Err = crate::types::error::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::AliasId::from_str(s)?.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasOutput {
    #[serde(with = "crate::types::stringify")]
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub alias_id: AliasId,
    pub state_index: u32,
    #[serde(with = "serde_bytes")]
    pub state_metadata: Box<[u8]>,
    pub foundry_counter: u32,
    pub unlock_conditions: Box<[UnlockCondition]>,
    pub features: Box<[Feature]>,
    pub immutable_features: Box<[Feature]>,
}

impl From<&bee::AliasOutput> for AliasOutput {
    fn from(value: &bee::AliasOutput) -> Self {
        Self {
            amount: value.amount(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            alias_id: (*value.alias_id()).into(),
            state_index: value.state_index(),
            state_metadata: value.state_metadata().to_vec().into_boxed_slice(),
            foundry_counter: value.foundry_counter(),
            unlock_conditions: value.unlock_conditions().iter().map(Into::into).collect(),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<AliasOutput> for bee::AliasOutput {
    type Error = crate::types::error::Error;

    fn try_from(value: AliasOutput) -> Result<Self, Self::Error> {
        Ok(Self::build_with_amount(value.amount, value.alias_id.try_into()?)?
            .with_native_tokens(
                Vec::from(value.native_tokens)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_state_index(value.state_index)
            .with_state_metadata(value.state_metadata.into())
            .with_foundry_counter(value.foundry_counter)
            .with_unlock_conditions(
                Vec::from(value.unlock_conditions)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
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
            .finish()?)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::block::output::{
        feature::test::{get_test_issuer_block, get_test_metadata_block, get_test_sender_block},
        native_token::test::get_test_native_token,
        unlock_condition::test::{get_test_governor_address_condition, get_test_state_controller_address_condition},
    };

    #[test]
    fn test_alias_id_bson() {
        let alias_id = AliasId::from(bee_test::rand::output::rand_alias_id());
        let bson = to_bson(&alias_id).unwrap();
        assert_eq!(alias_id, from_bson::<AliasId>(bson).unwrap());
    }

    #[test]
    fn test_alias_output_bson() {
        let output = get_test_alias_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<AliasOutput>(bson).unwrap());
    }

    pub(crate) fn get_test_alias_output() -> AliasOutput {
        AliasOutput::from(
            &bee::AliasOutput::build_with_amount(100, bee_test::rand::output::rand_alias_id())
                .unwrap()
                .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
                .with_state_index(0)
                .with_state_metadata("Foo".as_bytes().to_vec())
                .with_foundry_counter(0)
                .with_unlock_conditions(vec![
                    get_test_state_controller_address_condition(bee_test::rand::address::rand_address().into())
                        .try_into()
                        .unwrap(),
                    get_test_governor_address_condition(bee_test::rand::address::rand_address().into())
                        .try_into()
                        .unwrap(),
                ])
                .with_features(vec![
                    get_test_sender_block(bee_test::rand::address::rand_address().into())
                        .try_into()
                        .unwrap(),
                    get_test_metadata_block().try_into().unwrap(),
                ])
                .with_immutable_features(vec![
                    get_test_issuer_block(bee_test::rand::address::rand_address().into())
                        .try_into()
                        .unwrap(),
                    get_test_metadata_block().try_into().unwrap(),
                ])
                .finish()
                .unwrap(),
        )
    }
}
