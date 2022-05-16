// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_message_stardust::output as stardust;
use serde::{Deserialize, Serialize};

use super::{FeatureBlock, NativeToken, OutputAmount, UnlockCondition};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NftId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl NftId {
    pub fn from_output_id_str(s: &str) -> Result<Self, crate::types::error::Error> {
        Ok(stardust::NftId::from(stardust::OutputId::from_str(s)?).into())
    }
}

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

impl FromStr for NftId {
    type Err = crate::types::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(stardust::NftId::from_str(s)?.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NftOutput {
    amount: OutputAmount,
    native_tokens: Box<[NativeToken]>,
    nft_id: NftId,
    unlock_conditions: Box<[UnlockCondition]>,
    feature_blocks: Box<[FeatureBlock]>,
    immutable_feature_blocks: Box<[FeatureBlock]>,
}

impl From<&stardust::NftOutput> for NftOutput {
    fn from(value: &stardust::NftOutput) -> Self {
        Self {
            amount: value.amount(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            nft_id: (*value.nft_id()).into(),
            unlock_conditions: value.unlock_conditions().iter().map(Into::into).collect(),
            feature_blocks: value.feature_blocks().iter().map(Into::into).collect(),
            immutable_feature_blocks: value.immutable_feature_blocks().iter().map(Into::into).collect(),
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
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::message::output::{
        feature_block::test::{
            get_test_issuer_block, get_test_metadata_block, get_test_sender_block, get_test_tag_block,
        },
        native_token::test::get_test_native_token,
        unlock_condition::test::{
            get_test_address_condition, get_test_expiration_condition, get_test_storage_deposit_return_condition,
            get_test_timelock_condition,
        },
    };

    #[test]
    fn test_nft_id_bson() {
        let nft_id = NftId::from(rand_nft_id());
        let bson = to_bson(&nft_id).unwrap();
        assert_eq!(nft_id, from_bson::<NftId>(bson).unwrap());
    }

    #[test]
    fn test_nft_output_bson() {
        let output = get_test_nft_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<NftOutput>(bson).unwrap());
    }

    pub(crate) fn rand_nft_id() -> stardust::NftId {
        bee_test::rand::bytes::rand_bytes_array().into()
    }

    pub(crate) fn get_test_nft_output() -> NftOutput {
        NftOutput::from(
            &stardust::NftOutput::build_with_amount(100, rand_nft_id())
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
                .with_feature_blocks(vec![
                    get_test_sender_block(bee_test::rand::address::rand_address().into())
                        .try_into()
                        .unwrap(),
                    get_test_metadata_block().try_into().unwrap(),
                    get_test_tag_block().try_into().unwrap(),
                ])
                .with_immutable_feature_blocks(vec![
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
