// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use bee_block_stardust::payload::milestone as bee;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize, derive_more::Add, derive_more::Sub)]
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
