// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Defines types that allow for unified data processing.

mod ledger_updates;
mod milestone_stream;
mod sources;

use std::ops::RangeBounds;

use futures::{StreamExt, TryStreamExt};

pub use self::{
    ledger_updates::LedgerUpdateStore,
    milestone_stream::{Milestone, MilestoneStream},
    sources::{BlockData, InputSource, MilestoneData},
};
use crate::types::tangle::MilestoneIndex;

/// Provides access to the tangle.
pub struct Tangle<I: InputSource> {
    source: I,
}

impl<I: InputSource> Clone for Tangle<I> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
        }
    }
}
impl<I: InputSource + Copy> Copy for Tangle<I> {}

impl<I: InputSource> From<I> for Tangle<I> {
    fn from(source: I) -> Self {
        Self { source }
    }
}

impl<I: InputSource + Sync> Tangle<I> {
    /// Returns a stream of milestones for a given range.
    pub async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<MilestoneStream<'_, I>, I::Error> {
        let stream = self.source.milestone_stream(range).await?;
        Ok(MilestoneStream {
            inner: stream
                .and_then(|data| {
                    // #[allow(clippy::borrow_deref_ref)]
                    let source = self.source.clone();
                    async move {
                        Ok(Milestone {
                            ledger_updates: source.ledger_updates(data.at.milestone_index).await?,
                            source,
                            milestone_id: data.milestone_id,
                            at: data.at,
                            payload: data.payload,
                            protocol_params: data.protocol_params,
                            node_config: data.node_config,
                        })
                    }
                })
                .boxed(),
        })
    }
}
