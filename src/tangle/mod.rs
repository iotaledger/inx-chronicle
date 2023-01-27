// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Defines types that allow for unified data processing.

mod cone_stream;
mod ledger_updates;
mod milestone_stream;
mod sources;

use std::ops::RangeBounds;

use futures::{StreamExt, TryStreamExt};

pub use self::sources::{BlockData, MilestoneData};
use self::{
    milestone_stream::{Milestone, MilestoneStream},
    sources::InputSource,
};
use crate::types::tangle::MilestoneIndex;

/// Provides access to the tangle.
pub struct Tangle<'a, I: InputSource> {
    source: &'a I,
}

impl<'a, I: 'a + InputSource + Sync> Tangle<'a, I> {
    /// Returns a stream of milestones for a given range.
    pub async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<MilestoneStream<'a, I>, I::Error> {
        let stream = self.source.milestone_stream(range).await?;
        Ok(MilestoneStream {
            inner: stream
                .map_ok(|data| Milestone {
                    source: self.source,
                    milestone_id: data.milestone_id,
                    at: data.at,
                    payload: data.payload,
                    protocol_params: data.protocol_params,
                })
                .boxed(),
        })
    }
}
