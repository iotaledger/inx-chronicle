// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use derive_more::{Add, Deref, DerefMut, Sub};
use mongodb::bson::{doc, Bson};
use serde::{Deserialize, Serialize};

/// The Unix timestamp of a milestone.
#[derive(
    Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug, Default, Serialize, Deserialize, Add, Sub, Deref, DerefMut,
)]
#[serde(transparent)]
pub struct MilestoneTimestamp(pub u32);

impl From<u32> for MilestoneTimestamp {
    fn from(value: u32) -> Self {
        MilestoneTimestamp(value)
    }
}

impl From<MilestoneTimestamp> for Bson {
    fn from(value: MilestoneTimestamp) -> Self {
        Bson::from(value.0)
    }
}
