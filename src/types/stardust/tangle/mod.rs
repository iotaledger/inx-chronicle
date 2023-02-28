// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing Stardust Tangle models.

/// Module containing the block model.
pub mod block;
/// Module containing the milestone model.
pub mod milestone;

pub use self::milestone::{MilestoneIndex, MilestoneTimestamp};
