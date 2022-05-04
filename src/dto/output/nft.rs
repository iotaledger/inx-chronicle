// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output as stardust;
use serde::{Deserialize, Serialize};

use super::{FeatureBlock, NativeToken, OutputAmount, UnlockCondition};

pub type NftId = Box<[u8]>;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct NftOutput {
    amount: OutputAmount,
    native_tokens: Box<[NativeToken]>,
    nft_id: NftId,
    unlock_conditions: Box<[UnlockCondition]>,
    feature_blocks: Box<[FeatureBlock]>,
    immutable_feature_blocks: Box<[FeatureBlock]>,
}

impl From<stardust::NftOutput> for NftOutput {
    fn from(value: stardust::NftOutput) -> Self {
        // Self {
        //     amount: value.amount(),
        //     native_tokens: value.native_tokens().into_iter().collect::<Vec<_>>().into_boxed_slice(),
        //     nft_id: value.nft_id(),
        //     unlock_conditions: value.unlock_conditions(),
        //     feature_blocks: value.feature_blocks(),
        //     immutable_feature_blocks: value.immutable_feature_blocks(),
        // }
        todo!();
    }
}

impl TryFrom<NftOutput> for stardust::NftOutput {
    type Error = bee_message_stardust::Error;

    fn try_from(value: NftOutput) -> Result<Self, Self::Error> {
        // Self::new(value.amount)
        todo!();
    }
}
