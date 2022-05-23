// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt, ops};

use bee_block_stardust::payload::milestone as bee;
use derive_more::{Add, Deref, Sub};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Add, Sub, Deref)]
pub struct MilestoneIndex(pub u32);

impl fmt::Display for MilestoneIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for MilestoneIndex {
    fn from(value: u32) -> Self {
        MilestoneIndex(value)
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
