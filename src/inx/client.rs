// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{
    stream::{Stream, StreamExt},
    TryStreamExt,
};
use inx::{client::InxClient, proto};
use iota_sdk::types::block::{payload::signed_transaction::TransactionId, slot::SlotIndex};
use packable::PackableExt;

use super::{
    convert::TryConvertTo,
    ledger::{LedgerUpdate, UnspentOutput},
    request::SlotRangeRequest,
    InxError,
};
use crate::model::{
    block_metadata::{BlockWithMetadata, TransactionMetadata},
    node::{NodeConfiguration, NodeStatus},
    slot::Commitment,
};

/// An INX client connection.
#[derive(Clone, Debug)]
pub struct Inx {
    inx: InxClient<inx::tonic::transport::Channel>,
}

impl Inx {
    /// Connect to the INX interface of a node.
    pub async fn connect(address: &str) -> Result<Self, InxError> {
        Ok(Self {
            inx: InxClient::connect(address.to_owned()).await?,
        })
    }

    /// Get the status of the node.
    pub async fn get_node_status(&mut self) -> Result<NodeStatus, InxError> {
        self.inx.read_node_status(proto::NoParams {}).await?.try_convert()
    }

    /// Wait for the status of the node to change.
    pub async fn listen_to_status_changes(
        &mut self,
    ) -> Result<impl Stream<Item = Result<NodeStatus, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_node_status(proto::NodeStatusRequest {
                cooldown_in_milliseconds: 100,
            })
            .await?
            .into_inner()
            .map(|msg| msg?.try_convert()))
    }

    /// Get the configuration of the node.
    pub async fn get_node_configuration(&mut self) -> Result<NodeConfiguration, InxError> {
        self.inx
            .read_node_configuration(proto::NoParams {})
            .await?
            .try_convert()
    }

    /// Get a committed slot by index.
    pub async fn get_committed_slot(&mut self, slot: SlotIndex) -> Result<Commitment, InxError> {
        self.inx
            .read_commitment(proto::CommitmentRequest {
                commitment_slot: slot.0,
                commitment_id: None,
            })
            .await?
            .try_convert()
    }

    /// Get a stream of finalized slots.
    pub async fn get_finalized_slots(
        &mut self,
        request: SlotRangeRequest,
    ) -> Result<impl Stream<Item = Result<Commitment, InxError>>, InxError> {
        struct StreamState {
            inx: Option<Inx>,
            latest_finalized_slot: u32,
            curr_slot: u32,
            last_slot: u32,
        }

        let latest_finalized_slot = self
            .get_node_status()
            .await?
            .latest_finalized_commitment
            .commitment_id
            .slot_index()
            .0;
        Ok(futures::stream::unfold(
            StreamState {
                inx: Some(self.clone()),
                latest_finalized_slot,
                curr_slot: request.start_slot(),
                last_slot: request.end_slot(),
            },
            |mut state| async move {
                // Inner function definition to simplify result type
                async fn next(state: &mut StreamState) -> Result<Option<Commitment>, InxError> {
                    let Some(inx) = state.inx.as_mut() else { return Ok(None) };

                    if state.last_slot != 0 && state.curr_slot > state.last_slot {
                        return Ok(None);
                    }

                    // If the current slot is not yet finalized, we will wait.
                    if state.latest_finalized_slot < state.curr_slot {
                        let mut status_changes = inx.listen_to_status_changes().await?;
                        loop {
                            match status_changes.try_next().await? {
                                Some(status) => {
                                    // If the status change updated the latest finalized commitment, we can continue.
                                    if status.latest_finalized_commitment.commitment_id.slot_index().0
                                        > state.latest_finalized_slot
                                    {
                                        state.latest_finalized_slot =
                                            status.latest_finalized_commitment.commitment_id.slot_index().0;
                                        break;
                                    }
                                }
                                None => {
                                    return Ok(None);
                                }
                            }
                        }
                    }
                    let commitment = inx.get_committed_slot(state.curr_slot.into()).await?;
                    state.curr_slot += 1;
                    Ok(Some(commitment))
                }
                let res = next(&mut state).await;
                if res.is_err() {
                    state.inx = None;
                }
                res.transpose().map(|res| (res, state))
            },
        ))
    }

    /// Get accepted blocks for a given slot.
    pub async fn get_accepted_blocks_for_slot(
        &mut self,
        SlotIndex(slot): SlotIndex,
    ) -> Result<impl Stream<Item = Result<BlockWithMetadata, InxError>>, InxError> {
        Ok(self
            .inx
            .read_accepted_blocks(proto::SlotRequest { slot })
            .await?
            .into_inner()
            .map(|msg| msg?.try_convert()))
    }

    /// Get the associated metadata by transaction id.
    pub async fn get_transaction_metadata(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<TransactionMetadata, InxError> {
        self.inx
            .read_transaction_metadata(proto::TransactionId {
                id: transaction_id.pack_to_vec(),
            })
            .await?
            .into_inner()
            .try_convert()
    }

    /// Read the current unspent outputs.
    pub async fn get_unspent_outputs(
        &mut self,
    ) -> Result<impl Stream<Item = Result<UnspentOutput, InxError>>, InxError> {
        Ok(self
            .inx
            .read_unspent_outputs(proto::NoParams {})
            .await?
            .into_inner()
            .map(|msg| msg?.try_convert()))
    }

    /// Listen to ledger updates.
    pub async fn get_ledger_updates(
        &mut self,
        request: SlotRangeRequest,
    ) -> Result<impl Stream<Item = Result<LedgerUpdate, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_ledger_updates(proto::SlotRangeRequest::from(request))
            .await?
            .into_inner()
            .map(|msg| msg?.try_convert()))
    }
}
