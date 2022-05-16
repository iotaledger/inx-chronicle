// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as stardust;
use serde::{Deserialize, Serialize};

use super::{Feature, NativeToken, OutputAmount, UnlockCondition};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasicOutput {
    #[serde(with = "crate::types::stringify")]
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub unlock_conditions: Box<[UnlockCondition]>,
    pub features: Box<[Feature]>,
}

impl From<&stardust::BasicOutput> for BasicOutput {
    fn from(value: &stardust::BasicOutput) -> Self {
        Self {
            amount: value.amount(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            unlock_conditions: value.unlock_conditions().iter().map(Into::into).collect(),
            features: value.features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<BasicOutput> for stardust::BasicOutput {
    type Error = crate::types::error::Error;

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
