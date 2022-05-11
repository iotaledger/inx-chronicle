// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output as stardust;
use serde::{Deserialize, Serialize};

pub use super::{
    feature_block::FeatureBlock, native_token::NativeToken, unlock_condition::UnlockCondition, OutputAmount,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AliasId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

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
