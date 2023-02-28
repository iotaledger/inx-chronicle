// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing Stardust Milestone models.

/// Contains the `MilestoneIndex` type.
mod index;
/// Contains the `MilestoneTimestamp` type.
mod timestamp;

pub use self::{index::MilestoneIndex, timestamp::MilestoneTimestamp};
