// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as stardust;
use serde::{Deserialize, Serialize};

use super::{Feature, NativeToken, OutputAmount, UnlockCondition};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NftId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<stardust::NftId> for NftId {
    fn from(value: stardust::NftId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<NftId> for stardust::NftId {
    type Error = crate::types::error::Error;

    fn try_from(value: NftId) -> Result<Self, Self::Error> {
        Ok(stardust::NftId::new(value.0.as_ref().try_into()?))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NftOutput {
    amount: OutputAmount,
    native_tokens: Box<[NativeToken]>,
    nft_id: NftId,
    unlock_conditions: Box<[UnlockCondition]>,
    features: Box<[Feature]>,
    immutable_features: Box<[Feature]>,
}

impl From<&stardust::NftOutput> for NftOutput {
    fn from(value: &stardust::NftOutput) -> Self {
        Self {
            amount: value.amount(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            nft_id: (*value.nft_id()).into(),
            unlock_conditions: value.unlock_conditions().iter().map(Into::into).collect(),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<NftOutput> for stardust::NftOutput {
    type Error = crate::types::error::Error;

    fn try_from(value: NftOutput) -> Result<Self, Self::Error> {
        Ok(Self::build_with_amount(value.amount, value.nft_id.try_into()?)?
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
