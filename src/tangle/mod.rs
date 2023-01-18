// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Defines types that allow for unified data processing.

mod cone_stream;
mod milestone_stream;
mod ledger_updates;
mod sources;

use self::{cone_stream::ConeStream, sources::InputSource, milestone_stream::MilestoneStream};
use crate::types::tangle::MilestoneIndex;

/// Provides access to the tangle.
pub struct Tangle<I: InputSource> {
    source: I,
}

impl<'a, I: InputSource> Tangle<I> {
    /// Returns a stream of milestones for a given range.
    pub async fn milestone_stream(/* MilestoneRange */) -> Result<MilestoneStream<'a, I>, I::Error> {
        todo!();
    }
}
