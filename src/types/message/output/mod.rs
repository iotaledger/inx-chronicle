// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod feature_block;
mod native_token;
mod unlock_condition;

// The different output types
mod alias;
mod basic;
mod foundry;
mod nft;
mod treasury;

use bee_message_stardust::output as stardust;
use serde::{Deserialize, Serialize};

pub use self::{
    alias::{AliasId, AliasOutput},
    basic::BasicOutput,
    feature_block::FeatureBlock,
    foundry::FoundryOutput,
    native_token::{NativeToken, TokenScheme, TokenTag},
    nft::{NftId, NftOutput},
    treasury::TreasuryOutput,
    unlock_condition::UnlockCondition,
};
use crate::types::message::TransactionId;

pub type OutputAmount = u64;
pub type OutputIndex = u16;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputId {
    pub transaction_id: TransactionId,
    pub index: OutputIndex,
}

impl From<&stardust::OutputId> for OutputId {
    fn from(value: &stardust::OutputId) -> Self {
        Self {
            transaction_id: (*value.transaction_id()).into(),
            index: value.index(),
        }
    }
}

impl TryFrom<OutputId> for stardust::OutputId {
    type Error = crate::types::error::Error;

    fn try_from(value: OutputId) -> Result<Self, Self::Error> {
        Ok(stardust::OutputId::new(value.transaction_id.try_into()?, value.index)?)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

impl From<&stardust::Output> for Output {
    fn from(value: &stardust::Output) -> Self {
        match value {
            stardust::Output::Treasury(o) => Self::Treasury(o.into()),
            stardust::Output::Basic(o) => Self::Basic(o.into()),
            stardust::Output::Alias(o) => Self::Alias(o.into()),
            stardust::Output::Foundry(o) => Self::Foundry(o.into()),
            stardust::Output::Nft(o) => Self::Nft(o.into()),
        }
    }
}

impl TryFrom<Output> for stardust::Output {
    type Error = crate::types::error::Error;

    fn try_from(value: Output) -> Result<Self, Self::Error> {
        Ok(match value {
            Output::Treasury(o) => stardust::Output::Treasury(o.try_into()?),
            Output::Basic(o) => stardust::Output::Basic(o.try_into()?),
            Output::Alias(o) => stardust::Output::Alias(o.try_into()?),
            Output::Foundry(o) => stardust::Output::Foundry(o.try_into()?),
            Output::Nft(o) => stardust::Output::Nft(o.try_into()?),
        })
    }
}
