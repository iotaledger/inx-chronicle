// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::payload::milestone as bee;
use serde::{Deserialize, Serialize};

use crate::db;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MilestoneId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<bee::MilestoneId> for MilestoneId {
    fn from(value: bee::MilestoneId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<MilestoneId> for bee::MilestoneId {
    type Error = db::error::Error;

    fn try_from(value: MilestoneId) -> Result<Self, Self::Error> {
        Ok(bee::MilestoneId::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for MilestoneId {
    type Err = db::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::MilestoneId::from_str(s)?.into())
    }
}
