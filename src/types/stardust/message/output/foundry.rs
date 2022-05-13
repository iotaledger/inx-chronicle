// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output as stardust;
use serde::{Deserialize, Serialize};

use super::{FeatureBlock, NativeToken, OutputAmount, TokenScheme, TokenTag, UnlockCondition};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoundryOutput {
    #[serde(with = "crate::types::stringify")]
    amount: OutputAmount,
    native_tokens: Box<[NativeToken]>,
    #[serde(with = "crate::types::stringify")]
    serial_number: u32,
    token_tag: TokenTag,
    token_scheme: TokenScheme,
    unlock_conditions: Box<[UnlockCondition]>,
    feature_blocks: Box<[FeatureBlock]>,
    immutable_feature_blocks: Box<[FeatureBlock]>,
}

impl From<&stardust::FoundryOutput> for FoundryOutput {
    fn from(value: &stardust::FoundryOutput) -> Self {
        Self {
            amount: value.amount(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            serial_number: value.serial_number(),
            token_tag: value.token_tag().as_ref().to_vec().into_boxed_slice(),
            token_scheme: value.token_scheme().into(),
            unlock_conditions: value.unlock_conditions().iter().map(Into::into).collect(),
            feature_blocks: value.feature_blocks().iter().map(Into::into).collect(),
            immutable_feature_blocks: value.immutable_feature_blocks().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<FoundryOutput> for stardust::FoundryOutput {
    type Error = crate::types::error::Error;

    fn try_from(value: FoundryOutput) -> Result<Self, Self::Error> {
        Ok(Self::build_with_amount(
            value.amount,
            value.serial_number,
            stardust::TokenTag::new(value.token_tag.as_ref().try_into()?),
            value.token_scheme.try_into()?,
        )?
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
        .with_feature_blocks(
            Vec::from(value.feature_blocks)
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
        )
        .with_immutable_feature_blocks(
            Vec::from(value.immutable_feature_blocks)
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
        )
        .finish()?)
    }
}

#[cfg(test)]
pub(crate) mod test {
    pub(crate) const SERIAL_NUMBER: u32 = 12345;

    use std::str::FromStr;

    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::message::output::{
        alias::test::get_test_alias_id,
        feature_block::test::get_test_metadata_block,
        native_token::test::{get_test_native_token, TOKEN_TAG},
        unlock_condition::test::get_test_immut_alias_address_condition,
    };

    #[test]
    fn test_foundry_output_bson() {
        let output = get_test_foundry_output();
        let bson = to_bson(&output).unwrap();
        from_bson::<FoundryOutput>(bson).unwrap();
    }

    pub(crate) fn get_test_foundry_output() -> FoundryOutput {
        FoundryOutput::from(
            &stardust::FoundryOutput::build_with_amount(
                100,
                SERIAL_NUMBER,
                stardust::TokenTag::from_str(TOKEN_TAG).unwrap(),
                stardust::TokenScheme::Simple(
                    stardust::SimpleTokenScheme::new(250.into(), 200.into(), 300.into()).unwrap(),
                ),
            )
            .unwrap()
            .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
            .with_unlock_conditions(vec![
                get_test_immut_alias_address_condition(get_test_alias_id())
                    .try_into()
                    .unwrap(),
            ])
            .with_feature_blocks(vec![get_test_metadata_block().try_into().unwrap()])
            .with_immutable_feature_blocks(vec![get_test_metadata_block().try_into().unwrap()])
            .finish()
            .unwrap(),
        )
    }
}
