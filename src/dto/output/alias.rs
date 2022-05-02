// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

pub type AliasId = Box<[u8]>;

pub use super::{
    feature_block::FeatureBlock, native_token::NativeToken, unlock_condition::UnlockCondition, OutputAmount,
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AliasOutput {
    pub amount: OutputAmount,
    pub native_tokens: Box<NativeToken>,
    pub alias_id: AliasId,
    pub state_index: u32,
    pub state_metadata: Box<[u8]>,
    pub foundry_counter: u32,
    pub unlock_conditions: Box<[UnlockCondition]>,
    pub feature_blocks: Box<[FeatureBlock]>,
    pub immutable_feature_blocks: Box<[FeatureBlock]>,
}
