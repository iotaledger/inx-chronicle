// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Defines types that allow for unified data processing.

mod cone_stream;
mod milestone_stream;
mod input_source;
mod ledger_updates;
mod sources;

use self::{cone_stream::ConeStream, sources::InputSource};
use crate::types::tangle::MilestoneIndex;

/// Provides access to the tangle.
pub struct Tangle<I: InputSource> {
    source: I,
}

impl<'a, I: InputSource> Tangle<I> {
    pub async fn milestone_stream(/* MilestoneRange */) {}

    /// TODO: This function should probably be exposed on a `Milestone` object
    pub async fn cone_stream(index: MilestoneIndex) -> Result<ConeStream<'a, I>, I::Error> {
        todo!();
    }
}
