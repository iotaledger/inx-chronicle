// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`BlockId`] type.

use std::str::FromStr;

use iota_types::block as iota;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use crate::types::util::bytify;

/// Uniquely identifies a block.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Hash, Ord, PartialOrd, Eq)]
#[serde(transparent)]
pub struct BlockId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl BlockId {
    /// The number of bytes for the id.
    pub const LENGTH: usize = iota::BlockId::LENGTH;

    /// The `0x`-prefixed hex representation of a [`BlockId`].
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<iota::BlockId> for BlockId {
    fn from(value: iota::BlockId) -> Self {
        Self(*value)
    }
}

impl From<BlockId> for iota::BlockId {
    fn from(value: BlockId) -> Self {
        iota::BlockId::new(value.0)
    }
}

impl FromStr for BlockId {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::BlockId::from_str(s)?.into())
    }
}

impl From<BlockId> for Bson {
    fn from(val: BlockId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota::rand::block::{rand_block_id, rand_block_ids};

    use super::*;

    impl BlockId {
        /// Generates a random [`BlockId`].
        pub fn rand() -> Self {
            rand_block_id().into()
        }

        /// Generates multiple random [`BlockIds`](BlockId).
        pub fn rand_many(len: usize) -> impl Iterator<Item = Self> {
            rand_block_ids(len).into_iter().map(Into::into)
        }

        /// Generates a random amount of parents.
        pub fn rand_parents() -> Box<[Self]> {
            Self::rand_many(*iota::parent::Parents::COUNT_RANGE.end() as _).collect()
        }
    }
}
