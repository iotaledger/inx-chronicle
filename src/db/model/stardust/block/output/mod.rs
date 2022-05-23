// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod feature;
mod native_token;
mod unlock_condition;

// The different output types
mod alias;
mod basic;
mod foundry;
mod nft;
pub(crate) mod treasury;

use std::str::FromStr;

use bee_block_stardust::output as bee;
use serde::{Deserialize, Serialize};

pub use self::{
    alias::{AliasId, AliasOutput},
    basic::BasicOutput,
    feature::Feature,
    foundry::FoundryOutput,
    native_token::{NativeToken, TokenScheme},
    nft::{NftId, NftOutput},
    treasury::TreasuryOutput,
    unlock_condition::UnlockCondition,
};
use crate::db::model::stardust::block::TransactionId;

pub type OutputAmount = u64;
pub type OutputIndex = u16;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputId {
    pub transaction_id: TransactionId,
    pub index: OutputIndex,
}

impl From<bee::OutputId> for OutputId {
    fn from(value: bee::OutputId) -> Self {
        Self {
            transaction_id: (*value.transaction_id()).into(),
            index: value.index(),
        }
    }
}

impl TryFrom<OutputId> for bee::OutputId {
    type Error = crate::db::error::Error;

    fn try_from(value: OutputId) -> Result<Self, Self::Error> {
        Ok(bee::OutputId::new(value.transaction_id.try_into()?, value.index)?)
    }
}

impl FromStr for OutputId {
    type Err = crate::db::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::OutputId::from_str(s)?.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Output {
    #[serde(rename = "treasury")]
    Treasury(TreasuryOutput),
    #[serde(rename = "basic")]
    Basic(BasicOutput),
    #[serde(rename = "alias")]
    Alias(AliasOutput),
    #[serde(rename = "foundry")]
    Foundry(FoundryOutput),
    #[serde(rename = "nft")]
    Nft(NftOutput),
}

impl From<&bee::Output> for Output {
    fn from(value: &bee::Output) -> Self {
        match value {
            bee::Output::Treasury(o) => Self::Treasury(o.into()),
            bee::Output::Basic(o) => Self::Basic(o.into()),
            bee::Output::Alias(o) => Self::Alias(o.into()),
            bee::Output::Foundry(o) => Self::Foundry(o.into()),
            bee::Output::Nft(o) => Self::Nft(o.into()),
        }
    }
}

impl TryFrom<Output> for bee::Output {
    type Error = crate::db::error::Error;

    fn try_from(value: Output) -> Result<Self, Self::Error> {
        Ok(match value {
            Output::Treasury(o) => bee::Output::Treasury(o.try_into()?),
            Output::Basic(o) => bee::Output::Basic(o.try_into()?),
            Output::Alias(o) => bee::Output::Alias(o.try_into()?),
            Output::Foundry(o) => bee::Output::Foundry(o.try_into()?),
            Output::Nft(o) => bee::Output::Nft(o.try_into()?),
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::{alias, basic, foundry, nft, *};

    #[test]
    fn test_output_id_bson() {
        let output_id = OutputId::from(bee_test::rand::output::rand_output_id());
        let bson = to_bson(&output_id).unwrap();
        from_bson::<OutputId>(bson).unwrap();
    }

    #[test]
    fn test_output_bson() {
        let output = get_test_alias_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());

        let output = get_test_basic_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());

        let output = get_test_foundry_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());

        let output = get_test_nft_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());

        let output = Output::from(&bee::Output::Treasury(bee_test::rand::output::rand_treasury_output()));
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());
    }

    pub(crate) fn get_test_alias_output() -> Output {
        Output::Alias(alias::test::get_test_alias_output())
    }

    pub(crate) fn get_test_basic_output() -> Output {
        Output::Basic(basic::test::get_test_basic_output())
    }

    pub(crate) fn get_test_foundry_output() -> Output {
        Output::Foundry(foundry::test::get_test_foundry_output())
    }

    pub(crate) fn get_test_nft_output() -> Output {
        Output::Nft(nft::test::get_test_nft_output())
    }
}
