// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream};
use pin_project::pin_project;

use super::{ledger_updates::LedgerUpdateStore, sources::BlockData, InputSource};
use crate::types::{
    ledger::BlockMetadata,
    stardust::block::{payload::TransactionEssence, Block, BlockId, Input, Output, Payload},
};

/// A [`Block`] enriched with [`BlockMetadata`] and potentially [`Output`]s that correspond to inputs.
pub struct BlockWithMetadataInputs {
    pub block_id: BlockId,
    pub block: Block,
    pub raw: Vec<u8>,
    pub metadata: BlockMetadata,
    pub inputs: Vec<Output>,
}

#[pin_project]
pub struct ConeStream<'a, I: InputSource> {
    pub(super) store: LedgerUpdateStore,
    #[pin]
    pub(super) inner: BoxStream<'a, Result<BlockData, I::Error>>,
}

impl<'a, I: InputSource> Stream for ConeStream<'a, I> {
    type Item = Result<BlockWithMetadataInputs, I::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let mut input_vec = Vec::new();
        Pin::new(&mut this.inner).poll_next(cx).map_ok(|b| {
            // Enrich transaction payloads
            if let Some(payload) = b.block.payload.as_ref() {
                match payload {
                    Payload::Transaction(txn) => {
                        let TransactionEssence::Regular { inputs, .. } = &txn.essence;

                        for output_id in inputs.iter().filter_map(|input| match input {
                            Input::Utxo(output_id) => Some(*output_id),
                            _ => None,
                        }) {
                            input_vec.push(this.store.get_output(&output_id).unwrap().clone());
                        }
                    }
                    _ => (),
                }
            }
            BlockWithMetadataInputs {
                block_id: b.block_id,
                block: b.block,
                raw: b.raw,
                metadata: b.metadata,
                inputs: input_vec,
            }
        })
    }
}
