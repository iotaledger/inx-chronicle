// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub milestone_index: u32,
    pub logged: bool,
    pub synced: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncData {
    /// The completed(synced and logged) milestones data
    pub completed: Vec<Range<u32>>,
    /// Synced milestones data but unlogged
    pub synced_but_unlogged: Vec<Range<u32>>,
    /// Gaps/missings milestones data
    pub gaps: Vec<Range<u32>>,
}
