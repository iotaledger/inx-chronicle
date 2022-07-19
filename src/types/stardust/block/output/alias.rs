// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::output as bee;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use super::{
    feature::Feature,
    native_token::NativeToken,
    unlock_condition::{GovernorAddressUnlockCondition, StateControllerAddressUnlockCondition},
    OutputAmount,
};
use crate::types::util::bytify;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AliasId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl AliasId {
    const LENGTH: usize = bee::AliasId::LENGTH;

    pub fn from_output_id_str(s: &str) -> Result<Self, bee_block_stardust::Error> {
        Ok(bee::AliasId::from(bee::OutputId::from_str(s)?).into())
    }
}

impl From<bee::AliasId> for AliasId {
    fn from(value: bee::AliasId) -> Self {
        Self(*value)
    }
}

impl From<AliasId> for bee::AliasId {
    fn from(value: AliasId) -> Self {
        bee::AliasId::new(value.0)
    }
}

impl FromStr for AliasId {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::AliasId::from_str(s)?.into())
    }
}

impl From<AliasId> for Bson {
    fn from(val: AliasId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasOutput {
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub alias_id: AliasId,
    pub state_index: u32,
    #[serde(with = "serde_bytes")]
    pub state_metadata: Box<[u8]>,
    pub foundry_counter: u32,
    // The governor address unlock condition and the state controller unlock conditions are mandatory for now, but this
    // could change in the protocol in the future for compression reasons.
    pub state_controller_address_unlock_condition: StateControllerAddressUnlockCondition,
    pub governor_address_unlock_condition: GovernorAddressUnlockCondition,
    pub features: Box<[Feature]>,
    pub immutable_features: Box<[Feature]>,
}

impl From<&bee::AliasOutput> for AliasOutput {
    fn from(value: &bee::AliasOutput) -> Self {
        Self {
            amount: value.amount().into(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            alias_id: (*value.alias_id()).into(),
            state_index: value.state_index(),
            state_metadata: value.state_metadata().to_vec().into_boxed_slice(),
            foundry_counter: value.foundry_counter(),
            // Panic: The state controller address unlock condition has to be present for now.
            state_controller_address_unlock_condition: value
                .unlock_conditions()
                .state_controller_address()
                .unwrap()
                .into(),
            // Panic: The governor address unlock condition has to be present for now.
            governor_address_unlock_condition: value.unlock_conditions().governor_address().unwrap().into(),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<AliasOutput> for bee::AliasOutput {
    type Error = bee_block_stardust::Error;

    fn try_from(value: AliasOutput) -> Result<Self, Self::Error> {
        // The order of the conditions is important here because unlock conditions have to be sorted by type.
        let unlock_conditions = [
            Some(
                bee::unlock_condition::StateControllerAddressUnlockCondition::from(
                    value.state_controller_address_unlock_condition,
                )
                .into(),
            ),
            Some(
                bee::unlock_condition::GovernorAddressUnlockCondition::from(value.governor_address_unlock_condition)
                    .into(),
            ),
        ];

        Self::build_with_amount(value.amount.0, value.alias_id.into())?
            .with_native_tokens(
                Vec::from(value.native_tokens)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_state_index(value.state_index)
            .with_state_metadata(value.state_metadata.into())
            .with_foundry_counter(value.foundry_counter)
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

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::block::output::{
        feature::test::{get_test_issuer_block, get_test_metadata_block, get_test_sender_block},
        native_token::test::get_test_native_token,
        unlock_condition::test::{
            rand_governor_address_unlock_condition, rand_state_controller_address_unlock_condition,
        },
    };

    #[test]
    fn test_alias_id_bson() {
        let alias_id = AliasId::from(bee_test::rand::output::rand_alias_id());
        let bson = to_bson(&alias_id).unwrap();
        assert_eq!(Bson::from(alias_id), bson);
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
                .with_unlock_conditions([
                    rand_state_controller_address_unlock_condition().into(),
                    rand_governor_address_unlock_condition().into(),
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
