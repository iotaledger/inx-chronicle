// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output as stardust;
use serde::{Deserialize, Serialize};

use super::{FeatureBlock, NativeToken, OutputAmount, UnlockCondition};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasicOutput {
    #[serde(with = "crate::dto::stringify")]
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub unlock_conditions: Box<[UnlockCondition]>,
    pub feature_blocks: Box<[FeatureBlock]>,
}

impl From<&stardust::BasicOutput> for BasicOutput {
    fn from(value: &stardust::BasicOutput) -> Self {
        Self {
            amount: value.amount(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            unlock_conditions: value.unlock_conditions().iter().map(Into::into).collect(),
            feature_blocks: value.feature_blocks().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<BasicOutput> for stardust::BasicOutput {
    type Error = crate::dto::error::Error;

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
            .with_feature_blocks(
                Vec::from(value.feature_blocks)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .finish()?)
    }
}
