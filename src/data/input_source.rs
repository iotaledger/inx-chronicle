// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, StreamExt, TryStreamExt};

use crate::{
    db::{
        collections::{BlockCollection, MilestoneCollection, OutputCollection, ProtocolUpdateCollection},
        MongoDb,
    },
    types::{
        ledger::{BlockMetadata, MilestoneIndexTimestamp},
        stardust::block::{
            output::OutputId,
            payload::{MilestoneId, MilestonePayload, TransactionEssence},
            Block, BlockId, Input, Output, Payload,
        },
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

/// TODO: put this elsewhere and complete it
#[async_trait]
pub trait LedgerUpdateStore {
    /// The error type for this input source.
    type Error: std::error::Error + std::fmt::Debug;

    /// Get an output from the store.
    async fn get_output(&self, output_id: OutputId) -> Result<Option<Output>, Self::Error>;
}

/// Defines a type as a source for milestone and cone stream data.
#[async_trait]
pub trait InputSource {
    /// The type used to enrich transactions with inputs.
    type LedgerUpdateStore: 'static + LedgerUpdateStore<Error = Self::Error> + Send + Clone;
    /// The error type for this input source.
    type Error: 'static + std::error::Error + std::fmt::Debug;

    /// Retrieves a stream of milestones and their protocol parameters given a range of indexes.
    async fn milestone_stream(
        &self,
        range: std::ops::Range<MilestoneIndex>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<MilestoneAndProtocolParameters, Self::Error>>>>, Self::Error>;

    /// Retrieves a stream of blocks and their metadata in white-flag order given a milestone index.
    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<BlockWithMetadata, Self::Error>>>>, Self::Error>;

    /// Get the enriched cone stream using the ledger update store.
    async fn enriched_cone_stream(
        &self,
        index: MilestoneIndex,
        store: Self::LedgerUpdateStore,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<BlockWithMetadata, Self::Error>>>>, Self::Error> {
        Ok(Box::pin(self.cone_stream(index).await?.and_then(move |mut b| {
            let store = store.clone();
            async move {
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
                                input_vec.push(store.get_output(output_id).await?.unwrap());
                            }
                            b.inputs = Some(input_vec);
                        }
                        _ => (),
                    }
                }
                Ok(b)
            }
        })))
    }
}

#[allow(missing_docs)]
pub struct MilestoneAndProtocolParameters {
    pub milestone_id: MilestoneId,
    pub at: MilestoneIndexTimestamp,
    pub payload: MilestonePayload,
    pub protocol_params: ProtocolParameters,
}

#[allow(missing_docs)]
pub struct BlockWithMetadata {
    pub block_id: BlockId,
    pub block: Block,
    pub raw: Vec<u8>,
    pub metadata: BlockMetadata,
    pub inputs: Option<Vec<Output>>,
}

/// TODO: put this elsewhere and complete it
#[async_trait]
impl LedgerUpdateStore for MongoDb {
    type Error = mongodb::error::Error;

    async fn get_output(&self, output_id: OutputId) -> Result<Option<Output>, Self::Error> {
        self.collection::<OutputCollection>().get_output(&output_id).await
    }
}

#[async_trait]
impl InputSource for MongoDb {
    type LedgerUpdateStore = MongoDb;

    type Error = mongodb::error::Error;

    async fn milestone_stream(
        &self,
        range: std::ops::Range<MilestoneIndex>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<MilestoneAndProtocolParameters, Self::Error>>>>, Self::Error> {
        // Need to have an owned value to hold in the iterator
        let db = self.clone();
        Ok(Box::pin(futures::stream::iter(*range.start..*range.end).then(
            move |index| {
                let db = db.clone();
                async move {
                    let (milestone_id, at, payload) = db
                        .collection::<MilestoneCollection>()
                        .get_milestone(index.into())
                        .await?
                        // TODO: what do we do with this?
                        .unwrap();
                    let protocol_params = db
                        .collection::<ProtocolUpdateCollection>()
                        .get_protocol_parameters_for_ledger_index(index.into())
                        .await?
                        // TODO: what do we do with this?
                        .unwrap()
                        .parameters;
                    Ok(MilestoneAndProtocolParameters {
                        milestone_id,
                        at,
                        payload,
                        protocol_params,
                    })
                }
            },
        )))
    }

    /// Retrieves a stream of blocks and their metadata in white-flag order given a milestone index.
    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<BlockWithMetadata, Self::Error>>>>, Self::Error> {
        Ok(Box::pin(
            self.collection::<BlockCollection>()
                .get_referenced_blocks_in_white_flag_order_stream(index)
                .await?
                .map_ok(|(block_id, block, raw, metadata)| BlockWithMetadata {
                    block_id,
                    block,
                    raw,
                    metadata,
                    inputs: None,
                }),
        ))
    }
}
