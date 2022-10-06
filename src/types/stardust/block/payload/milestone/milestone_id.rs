// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::payload::milestone as bee;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use crate::types::util::bytify;

/// Uniquely identifies a milestone.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MilestoneId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl MilestoneId {
    const LENGTH: usize = bee::MilestoneId::LENGTH;

    /// Converts the [`MilestoneId`] to its `0x`-prefixed hex representation.
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<bee::MilestoneId> for MilestoneId {
    fn from(value: bee::MilestoneId) -> Self {
        Self(*value)
    }
}

impl From<MilestoneId> for bee::MilestoneId {
    fn from(value: MilestoneId) -> Self {
        bee::MilestoneId::new(value.0)
    }
}

impl FromStr for MilestoneId {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::MilestoneId::from_str(s)?.into())
    }
}

impl From<MilestoneId> for Bson {
    fn from(val: MilestoneId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::milestone::rand_milestone_id;

    use super::*;

    impl MilestoneId {
        /// Generates a random [`MilestoneId`].
        pub fn rand() -> Self {
            rand_milestone_id().into()
        }
    }
}
