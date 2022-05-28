// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust as bee;
use serde::{Deserialize, Serialize};

use crate::types::util::bytify;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Hash, Ord, PartialOrd, Eq)]
#[serde(transparent)]
pub struct BlockId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl BlockId {
    const LENGTH: usize = bee::BlockId::LENGTH;

    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<bee::BlockId> for BlockId {
    fn from(value: bee::BlockId) -> Self {
        Self(*value)
    }
}

impl From<BlockId> for bee::BlockId {
    fn from(value: BlockId) -> Self {
        bee::BlockId::new(value.0)
    }
}

impl FromStr for BlockId {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::BlockId::from_str(s)?.into())
    }
}
