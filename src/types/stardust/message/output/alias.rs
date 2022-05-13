// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_message_stardust::output as stardust;
use serde::{Deserialize, Serialize};

use super::{feature_block::FeatureBlock, native_token::NativeToken, unlock_condition::UnlockCondition, OutputAmount};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AliasId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl AliasId {
    pub fn from_output_id_str(s: &str) -> Result<Self, crate::types::error::Error> {
        Ok(stardust::AliasId::from(stardust::OutputId::from_str(s)?).into())
    }
}

impl From<stardust::AliasId> for AliasId {
    fn from(value: stardust::AliasId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<AliasId> for stardust::AliasId {
    type Error = crate::types::error::Error;

    fn try_from(value: AliasId) -> Result<Self, Self::Error> {
        Ok(stardust::AliasId::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for AliasId {
    type Err = crate::types::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(stardust::AliasId::from_str(s)?.into())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    pub feature_blocks: Box<[FeatureBlock]>,
    pub immutable_feature_blocks: Box<[FeatureBlock]>,
}

impl From<&stardust::AliasOutput> for AliasOutput {
    fn from(value: &stardust::AliasOutput) -> Self {
        Self {
            amount: value.amount(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            alias_id: (*value.alias_id()).into(),
            state_index: value.state_index(),
            state_metadata: value.state_metadata().to_vec().into_boxed_slice(),
            foundry_counter: value.foundry_counter(),
            unlock_conditions: value.unlock_conditions().iter().map(Into::into).collect(),
            feature_blocks: value.feature_blocks().iter().map(Into::into).collect(),
            immutable_feature_blocks: value.immutable_feature_blocks().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<AliasOutput> for stardust::AliasOutput {
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
    pub(crate) const STATE_METADATA: &str = "Foo";

    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::message::{
        address::test::{get_test_ed25519_address, get_test_nft_address},
        output::{
            feature_block::test::{get_test_issuer_block, get_test_metadata_block, get_test_sender_block},
            native_token::test::get_test_native_token,
            test::OUTPUT_ID,
            unlock_condition::test::{
                get_test_governor_address_condition, get_test_state_controller_address_condition,
            },
        },
        Address,
    };

    #[test]
    fn test_alias_id_bson() {
        let alias_id = get_test_alias_id();
        let bson = to_bson(&alias_id).unwrap();
        from_bson::<AliasId>(bson).unwrap();
    }

    #[test]
    fn test_alias_output_bson() {
        let output = get_test_alias_output();
        let bson = to_bson(&output).unwrap();
        from_bson::<AliasOutput>(bson).unwrap();
    }

    pub(crate) fn get_test_alias_id() -> AliasId {
        AliasId::from_output_id_str(OUTPUT_ID).unwrap()
    }

    pub(crate) fn get_test_alias_output() -> AliasOutput {
        AliasOutput::from(
            &stardust::AliasOutput::build_with_amount(100, get_test_alias_id().try_into().unwrap())
                .unwrap()
                .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
                .with_state_index(0)
                .with_state_metadata(STATE_METADATA.as_bytes().to_vec())
                .with_foundry_counter(0)
                .with_unlock_conditions(vec![
                    get_test_state_controller_address_condition(get_test_ed25519_address())
                        .try_into()
                        .unwrap(),
                    get_test_governor_address_condition(get_alt_test_alias_address())
                        .try_into()
                        .unwrap(),
                ])
                .with_feature_blocks(vec![
                    get_test_sender_block(get_test_nft_address()).try_into().unwrap(),
                    get_test_metadata_block().try_into().unwrap(),
                ])
                .with_immutable_feature_blocks(vec![
                    get_test_issuer_block(get_alt_test_alias_address()).try_into().unwrap(),
                    get_test_metadata_block().try_into().unwrap(),
                ])
                .finish()
                .unwrap(),
        )
    }

    fn get_alt_test_alias_address() -> Address {
        Address::Alias(
            AliasId::from_output_id_str("0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c6492b00")
                .unwrap(),
        )
    }
}
