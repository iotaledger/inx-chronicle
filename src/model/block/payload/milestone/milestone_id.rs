// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_types::block::payload::milestone as iota;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use crate::model::bytify;

/// Uniquely identifies a milestone.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct MilestoneId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl MilestoneId {
    /// The number of bytes for the id.
    pub const LENGTH: usize = iota::MilestoneId::LENGTH;

    /// Converts the [`MilestoneId`] to its `0x`-prefixed hex representation.
    pub fn to_hex(&self) -> String {
        prefix_hex::encode(self.0.as_ref())
    }
}

impl From<iota::MilestoneId> for MilestoneId {
    fn from(value: iota::MilestoneId) -> Self {
        Self(*value)
    }
}

impl From<MilestoneId> for iota::MilestoneId {
    fn from(value: MilestoneId) -> Self {
        iota::MilestoneId::new(value.0)
    }
}

impl FromStr for MilestoneId {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::MilestoneId::from_str(s)?.into())
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
    use iota_types::block::rand::milestone::rand_milestone_id;

    use super::*;

    impl MilestoneId {
        /// Generates a random [`MilestoneId`].
        pub fn rand() -> Self {
            rand_milestone_id().into()
        }
    }
}
