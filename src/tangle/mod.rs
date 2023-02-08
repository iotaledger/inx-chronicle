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
use crate::{db::MongoDb, types::tangle::MilestoneIndex};

/// Provides access to the tangle.
pub struct Tangle<I: InputSource + 'static> {
    source: I,
}

impl<I: InputSource + Clone> Clone for Tangle<I> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
        }
    }
}
impl<I: InputSource + Copy> Copy for Tangle<I> {}

impl Tangle<MongoDb> {
    /// Create a tangle from a [`MongoDb`] input source.
    pub fn from_mongodb(mongodb: MongoDb) -> Self {
        Self { source: mongodb }
    }
}

#[cfg(feature = "inx")]
impl Tangle<crate::inx::Inx> {
    /// Create a tangle from an [`Inx`](crate::inx::Inx) input source.
    pub fn from_inx(inx: crate::inx::Inx) -> Self {
        Self { source: inx }
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
                    #[allow(clippy::borrow_deref_ref)]
                    let source = &self.source;
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
