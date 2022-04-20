// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use serde::{Deserialize, Serialize};

/// A record indicating that a milestone is completed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    /// The index of the milestone that was completed.
    pub milestone_index: u32,
    /// Whether the milestone has been written to an archive file.
    pub logged: bool,
    /// Whether the milestone has been synced.
    pub synced: bool,
}

/// An aggregation type that represents the ranges of completed milestones and gaps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncData {
    /// The completed(synced and logged) milestones data
    pub completed: Vec<Range<u32>>,
    /// Synced milestones data but unlogged
    pub synced_but_unlogged: Vec<Range<u32>>,
    /// Gaps/missings milestones data
    pub gaps: Vec<Range<u32>>,
}
