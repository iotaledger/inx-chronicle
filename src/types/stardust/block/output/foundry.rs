// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as stardust;
use serde::{Deserialize, Serialize};

use super::{Feature, NativeToken, OutputAmount, TokenScheme, UnlockCondition};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoundryOutput {
    #[serde(with = "crate::types::stringify")]
    amount: OutputAmount,
    native_tokens: Box<[NativeToken]>,
    #[serde(with = "crate::types::stringify")]
    serial_number: u32,
    token_scheme: TokenScheme,
    unlock_conditions: Box<[UnlockCondition]>,
    features: Box<[Feature]>,
    immutable_features: Box<[Feature]>,
}

impl From<&stardust::FoundryOutput> for FoundryOutput {
    fn from(value: &stardust::FoundryOutput) -> Self {
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

impl TryFrom<FoundryOutput> for stardust::FoundryOutput {
    type Error = crate::types::error::Error;

    fn try_from(value: FoundryOutput) -> Result<Self, Self::Error> {
        Ok(Self::build_with_amount(
            value.amount,
            value.serial_number,
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
