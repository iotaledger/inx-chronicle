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
pub struct Tangle<'a, I: InputSource> {
    source: &'a I,
}

impl<'a, I: InputSource> Clone for Tangle<'a, I> {
    fn clone(&self) -> Self {
        Self { source: self.source }
    }
}
impl<'a, I: InputSource> Copy for Tangle<'a, I> {}

impl<'a, I: InputSource> From<&'a I> for Tangle<'a, I> {
    fn from(source: &'a I) -> Self {
        Self { source }
    }
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
                .and_then(|data| {
                    #[allow(clippy::borrow_deref_ref)]
                    let source = &*self.source;
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
