// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::payload::milestone as bee;
use serde::{Deserialize, Serialize};

use crate::types::util::bytify;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MilestoneId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl MilestoneId {
    const LENGTH: usize = bee::MilestoneId::LENGTH;
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
    type Err = crate::types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::MilestoneId::from_str(s)?.into())
    }
}
