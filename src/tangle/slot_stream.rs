// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream};
use iota_sdk::types::block::{
    protocol::ProtocolParameters,
    slot::{SlotCommitment, SlotCommitmentId, SlotIndex},
};

use super::InputSource;
use crate::model::{block_metadata::BlockWithMetadata, ledger::LedgerUpdateStore, raw::Raw, slot::Commitment};

#[allow(missing_docs)]
pub struct Slot<'a, I: InputSource> {
    pub(super) source: &'a I,
    pub commitment: Commitment,
    pub protocol_parameters: ProtocolParameters,
    pub ledger_updates: LedgerUpdateStore,
}

impl<'a, I: InputSource> Slot<'a, I> {
    /// Get the slot's index.
    pub fn index(&self) -> SlotIndex {
        self.commitment.commitment_id.slot_index()
    }

    /// Get the slot's commitment id.
    pub fn commitment_id(&self) -> SlotCommitmentId {
        self.commitment.commitment_id
    }

    /// Get the slot's raw commitment.
    pub fn commitment(&self) -> &Raw<SlotCommitment> {
        &self.commitment.commitment
    }
}

impl<'a, I: InputSource> Slot<'a, I> {
    /// Returns the accepted blocks of a slot.
    pub async fn accepted_block_stream(&self) -> Result<BoxStream<Result<BlockWithMetadata, I::Error>>, I::Error> {
        self.source.accepted_blocks(self.index()).await
    }

    /// Returns the ledger update store.
    pub fn ledger_updates(&self) -> &LedgerUpdateStore {
        &self.ledger_updates
    }
}

#[allow(missing_docs)]
pub struct SlotStream<'a, I: InputSource> {
    pub(super) inner: BoxStream<'a, Result<Slot<'a, I>, I::Error>>,
}

impl<'a, I: InputSource> Stream for SlotStream<'a, I> {
    type Item = Result<Slot<'a, I>, I::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
}
