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

use serde::{Deserialize, Serialize};

pub use self::{
    alias::AliasOutput,
    basic::BasicOutput,
    feature_block::FeatureBlock,
    foundry::FoundryOutput,
    native_token::{NativeToken, TokenScheme, TokenTag},
    nft::NftOutput,
    treasury::TreasuryOutput,
    unlock_condition::UnlockCondition,
};
use crate::dto;

pub type OutputAmount = u64;
pub type OutputIndex = u16;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputId {
    transaction_id: dto::TransactionId,
    index: OutputIndex,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Output {
    Treasury(TreasuryOutput),
    Basic(BasicOutput),
    Alias(AliasOutput),
    Foundry(FoundryOutput),
    Nft(NftOutput),
}
