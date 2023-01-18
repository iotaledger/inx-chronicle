// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::BoxStream, Stream};
use pin_project::pin_project;

use super::{ledger_updates::LedgerUpdateStore, InputSource};
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
    pub inputs: Option<Vec<Output>>,
}

#[pin_project]
pub struct ConeStream<'a, I: InputSource> {
    store: &'a LedgerUpdateStore,
    #[pin]
    inner: BoxStream<'a, Result<BlockWithMetadataInputs, I::Error>>,
}

impl<'a, I: InputSource> Stream for ConeStream<'a, I> {
    type Item = Result<BlockWithMetadataInputs, I::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        Pin::new(&mut this.inner).poll_next(cx).map_ok(|mut b| {
            // Enrich transaction payloads
            if let Some(payload) = b.block.payload.as_ref() {
                match payload {
                    Payload::Transaction(txn) => {
                        let TransactionEssence::Regular { inputs, .. } = &txn.essence;
                        let mut input_vec = Vec::new();
                        for output_id in inputs.iter().filter_map(|input| match input {
                            Input::Utxo(output_id) => Some(*output_id),
                            _ => None,
                        }) {
                            input_vec.push(this.store.get_output(&output_id).unwrap().clone());
                        }
                        b.inputs = Some(input_vec);
                    }
                    _ => (),
                }
            }
            b
        })
    }
}
