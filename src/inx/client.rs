// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::stream::{Stream, StreamExt};
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

    /// Get the configuration of the node.
    pub async fn get_node_configuration(&mut self) -> Result<NodeConfiguration, InxError> {
        self.inx
            .read_node_configuration(proto::NoParams {})
            .await?
            .try_convert()
    }

    /// Get a stream of committed slots.
    pub async fn get_committed_slots(
        &mut self,
        request: SlotRangeRequest,
    ) -> Result<impl Stream<Item = Result<Commitment, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_commitments(proto::SlotRangeRequest::from(request))
            .await?
            .into_inner()
            .map(|msg| msg?.try_convert()))
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
