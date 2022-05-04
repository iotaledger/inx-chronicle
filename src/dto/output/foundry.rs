// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::{FeatureBlock, NativeToken, OutputAmount, TokenScheme, TokenTag, UnlockCondition};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FoundryOutput {
    amount: OutputAmount,
    native_tokens: Box<[NativeToken]>,
    serial_number: u32,
    token_tag: TokenTag,
    token_scheme: TokenScheme,
    unlock_conditions: Box<[UnlockCondition]>,
    feature_blocks: Box<[FeatureBlock]>,
    immutable_feature_blocks: Box<[FeatureBlock]>,
}
