// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Defines types that allow for unified data processing.

mod cone_stream;
mod ledger_updates;
mod milestone_range;
mod milestone_stream;
mod sources;

use self::{milestone_range::MilestoneRange, milestone_stream::MilestoneStream, sources::InputSource};

/// Provides access to the tangle.
pub struct Tangle<I: InputSource> {
    source: I,
}

impl<'a, I: 'a + InputSource> Tangle<I> {
    /// Returns a stream of milestones for a given range.
    pub async fn milestone_stream(&self, range: impl Into<MilestoneRange>) -> Result<MilestoneStream<'a, I>, I::Error> {
        todo!()
    }
}
