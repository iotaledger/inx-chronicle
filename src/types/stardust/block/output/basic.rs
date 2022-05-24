// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as bee;
use serde::{Deserialize, Serialize};

use super::{Feature, NativeToken, OutputAmount, UnlockCondition};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicOutput {
    #[serde(with = "crate::types::util::stringify")]
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub unlock_conditions: Box<[UnlockCondition]>,
    pub features: Box<[Feature]>,
}

impl From<&bee::BasicOutput> for BasicOutput {
    fn from(value: &bee::BasicOutput) -> Self {
        Self {
            amount: value.amount(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            unlock_conditions: value.unlock_conditions().iter().map(Into::into).collect(),
            features: value.features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<BasicOutput> for bee::BasicOutput {
    type Error = crate::types::Error;

    fn try_from(value: BasicOutput) -> Result<Self, Self::Error> {
        Ok(Self::build_with_amount(value.amount)?
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
            .finish()?)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::block::output::{
        feature::test::{get_test_metadata_block, get_test_sender_block, get_test_tag_block},
        native_token::test::get_test_native_token,
        unlock_condition::test::{
            get_test_address_condition, get_test_expiration_condition, get_test_storage_deposit_return_condition,
            get_test_timelock_condition,
        },
    };

    #[test]
    fn test_basic_output_bson() {
        let output = get_test_basic_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<BasicOutput>(bson).unwrap());
    }

    pub(crate) fn get_test_basic_output() -> BasicOutput {
        BasicOutput::from(
            &bee::BasicOutput::build_with_amount(100)
                .unwrap()
                .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
                .with_unlock_conditions(vec![
                    get_test_address_condition(bee_test::rand::address::rand_address().into())
                        .try_into()
                        .unwrap(),
                    get_test_storage_deposit_return_condition(bee_test::rand::address::rand_address().into(), 1)
                        .try_into()
                        .unwrap(),
                    get_test_timelock_condition(1, 1).try_into().unwrap(),
                    get_test_expiration_condition(bee_test::rand::address::rand_address().into(), 1, 1)
                        .try_into()
                        .unwrap(),
                ])
                .with_features(vec![
                    get_test_sender_block(bee_test::rand::address::rand_address().into())
                        .try_into()
                        .unwrap(),
                    get_test_metadata_block().try_into().unwrap(),
                    get_test_tag_block().try_into().unwrap(),
                ])
                .finish()
                .unwrap(),
        )
    }
}
