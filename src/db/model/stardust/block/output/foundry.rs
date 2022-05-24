// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as bee;
use serde::{Deserialize, Serialize};

use super::{Feature, NativeToken, OutputAmount, TokenScheme, UnlockCondition};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryOutput {
    #[serde(with = "crate::db::model::util::stringify")]
    amount: OutputAmount,
    native_tokens: Box<[NativeToken]>,
    #[serde(with = "crate::db::model::util::stringify")]
    serial_number: u32,
    token_scheme: TokenScheme,
    unlock_conditions: Box<[UnlockCondition]>,
    features: Box<[Feature]>,
    immutable_features: Box<[Feature]>,
}

impl From<&bee::FoundryOutput> for FoundryOutput {
    fn from(value: &bee::FoundryOutput) -> Self {
        Self {
            amount: value.amount(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            serial_number: value.serial_number(),
            token_scheme: value.token_scheme().into(),
            unlock_conditions: value.unlock_conditions().iter().map(Into::into).collect(),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<FoundryOutput> for bee::FoundryOutput {
    type Error = crate::db::error::Error;

    fn try_from(value: FoundryOutput) -> Result<Self, Self::Error> {
        Ok(
            Self::build_with_amount(value.amount, value.serial_number, value.token_scheme.try_into()?)?
                .with_native_tokens(
                    Vec::from(value.native_tokens)
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<Vec<_>, _>>()?,
                )
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
                .finish()?,
        )
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::db::model::stardust::block::output::{
        feature::test::get_test_metadata_block,
        native_token::test::get_test_native_token,
        unlock_condition::test::{get_test_alias_address_as_address, get_test_immut_alias_address_condition},
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
                bee_test::rand::number::rand_number(),
                bee::TokenScheme::Simple(bee::SimpleTokenScheme::new(250.into(), 200.into(), 300.into()).unwrap()),
            )
            .unwrap()
            .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
            .with_unlock_conditions(vec![
                get_test_immut_alias_address_condition(get_test_alias_address_as_address())
                    .try_into()
                    .unwrap(),
            ])
            .with_features(vec![get_test_metadata_block().try_into().unwrap()])
            .with_immutable_features(vec![get_test_metadata_block().try_into().unwrap()])
            .finish()
            .unwrap(),
        )
    }
}
