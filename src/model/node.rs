// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the node models.

use core::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::tangle::MilestoneIndex;

/// The [`NodeConfiguration`] type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct NodeConfiguration {
    pub milestone_public_key_count: u32,
    pub milestone_key_ranges: Box<[MilestoneKeyRange]>,
    pub base_token: BaseToken,
}

/// The [`BaseToken`] type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BaseToken {
    pub name: String,
    pub ticker_symbol: String,
    pub unit: String,
    pub subunit: String,
    pub decimals: u32,
    pub use_metric_prefix: bool,
}

/// The [`MilestoneKeyRange`] type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MilestoneKeyRange {
    pub public_key: String,
    pub start: MilestoneIndex,
    pub end: MilestoneIndex,
}

impl Ord for MilestoneKeyRange {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}

impl PartialOrd for MilestoneKeyRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
