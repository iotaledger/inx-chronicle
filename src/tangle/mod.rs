// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Defines types that allow for unified data processing.

mod slot_stream;
pub(crate) mod sources;
use std::ops::RangeBounds;

use futures::{StreamExt, TryStreamExt};
use iota_sdk::types::block::slot::SlotIndex;

pub use self::{
    slot_stream::{Slot, SlotStream},
    sources::InputSource,
};

/// Provides access to the tangle.
pub struct Tangle<I: InputSource> {
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

impl<I: InputSource> From<I> for Tangle<I> {
    fn from(source: I) -> Self {
        Self { source }
    }
}

impl<I: InputSource + Sync> Tangle<I> {
    /// Returns a stream of slots in a given range.
    pub async fn slot_stream(&self, range: impl RangeBounds<SlotIndex> + Send) -> Result<SlotStream<'_, I>, I::Error> {
        let stream = self.source.slot_stream(range).await?;
        Ok(SlotStream {
            inner: stream
                .and_then(|data| {
                    #[allow(clippy::borrow_deref_ref)]
                    let source = &self.source;
                    async move {
                        Ok(Slot {
                            ledger_updates: source
                                .ledger_updates(data.commitment.commitment_id.slot_index())
                                .await?,
                            source,
                            protocol_params: data.node_config.protocol_parameters.last().unwrap().clone(),
                            node_config: data.node_config,
                            commitment: data.commitment,
                        })
                    }
                })
                .boxed(),
        })
    }
}
