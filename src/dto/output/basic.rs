// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::{FeatureBlock, NativeToken, OutputAmount, UnlockCondition};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BasicOutput {
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub unlock_conditions: Box<[UnlockCondition]>,
    pub feature_blocks: Box<[FeatureBlock]>,
}
