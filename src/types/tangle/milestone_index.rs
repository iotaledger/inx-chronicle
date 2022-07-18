// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt, num::ParseIntError, ops, str::FromStr};

use bee_block_stardust::payload::milestone as bee;
use derive_more::{Add, Deref, DerefMut, Sub};
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Add, Sub, Deref, DerefMut,
)]
#[serde(transparent)]
pub struct MilestoneIndex(pub u32);

impl fmt::Display for MilestoneIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<u32> for MilestoneIndex {
    fn from(value: u32) -> Self {
        MilestoneIndex(value)
    }
}

impl From<MilestoneIndex> for u32 {
    fn from(value: MilestoneIndex) -> Self {
        value.0
    }
}

impl ops::Add<u32> for MilestoneIndex {
    type Output = Self;

    fn add(self, x: u32) -> Self {
        MilestoneIndex(self.0 + x)
    }
}

impl ops::AddAssign<u32> for MilestoneIndex {
    fn add_assign(&mut self, x: u32) {
        self.0 += x
    }
}

impl ops::Sub<u32> for MilestoneIndex {
    type Output = Self;

    fn sub(self, x: u32) -> Self {
        MilestoneIndex(self.0 - x)
    }
}

impl PartialEq<u32> for MilestoneIndex {
    fn eq(&self, x: &u32) -> bool {
        self.0 == *x
    }
}

impl PartialEq<MilestoneIndex> for u32 {
    fn eq(&self, x: &MilestoneIndex) -> bool {
        *self == x.0
    }
}

impl From<bee::MilestoneIndex> for MilestoneIndex {
    fn from(value: bee::MilestoneIndex) -> Self {
        Self(value.0)
    }
}

impl From<MilestoneIndex> for bee::MilestoneIndex {
    fn from(value: MilestoneIndex) -> Self {
        Self(value.0)
    }
}

impl From<MilestoneIndex> for Bson {
    fn from(value: MilestoneIndex) -> Self {
        Bson::from(value.0)
    }
}

impl FromStr for MilestoneIndex {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(u32::from_str(s)?.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn add_assign() {
        let mut a = MilestoneIndex(42);
        a += 1;
        assert_eq!(a, MilestoneIndex(43))
    }
}
