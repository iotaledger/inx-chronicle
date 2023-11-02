// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{
    core::validation::{ShallowLikeParents, StrongParents, WeakParents},
    protocol::ProtocolParametersHash,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidationBlockDto {
    /// Blocks that are strongly directly approved.
    strong_parents: StrongParents,
    /// Blocks that are weakly directly approved.
    weak_parents: WeakParents,
    /// Blocks that are directly referenced to adjust opinion.
    shallow_like_parents: ShallowLikeParents,
    /// The highest supported protocol version the issuer of this block supports.
    highest_supported_version: u8,
    /// The hash of the protocol parameters for the Highest Supported Version.
    protocol_parameters_hash: ProtocolParametersHash,
}
