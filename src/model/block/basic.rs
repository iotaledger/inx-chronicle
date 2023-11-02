// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::core::basic::{ShallowLikeParents, StrongParents, WeakParents};
use serde::{Deserialize, Serialize};

use super::payload::PayloadDto;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BasicBlockDto {
    /// Blocks that are strongly directly approved.
    strong_parents: StrongParents,
    /// Blocks that are weakly directly approved.
    weak_parents: WeakParents,
    /// Blocks that are directly referenced to adjust opinion.
    shallow_like_parents: ShallowLikeParents,
    /// The optional [`Payload`] of the block.
    payload: Option<PayloadDto>,
    /// The amount of Mana the Account identified by [`IssuerId`](super::IssuerId) is at most willing to burn for this
    /// block.
    max_burned_mana: u64,
}
