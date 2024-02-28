// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::stream::{Stream, StreamExt};
use inx::{client::InxClient, proto};
use iota_sdk::types::block::slot::SlotIndex;

use super::{
    convert::TryConvertTo,
    ledger::{LedgerUpdate, UnspentOutput},
    request::SlotRangeRequest,
    InxError,
};
use crate::model::{
    block_metadata::BlockWithMetadata,
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
            .map(|msg| TryConvertTo::try_convert(msg?)))
    }

    /// Convenience wrapper that gets accepted blocks for a given slot.
    pub async fn get_accepted_blocks_for_slot(
        &mut self,
        slot_index: SlotIndex,
    ) -> Result<impl Stream<Item = Result<BlockWithMetadata, InxError>>, InxError> {
        Ok(self
            .inx
            .read_accepted_blocks(proto::SlotIndex { index: slot_index.0 })
            .await?
            .into_inner()
            .map(|msg| TryConvertTo::try_convert(msg?)))
    }

    /// Convenience wrapper that reads the current unspent outputs.
    pub async fn get_unspent_outputs(
        &mut self,
    ) -> Result<impl Stream<Item = Result<UnspentOutput, InxError>>, InxError> {
        Ok(self
            .inx
            .read_unspent_outputs(proto::NoParams {})
            .await?
            .into_inner()
            .map(|msg| TryConvertTo::try_convert(msg?)))
    }

    /// Convenience wrapper that listen to ledger updates.
    pub async fn get_ledger_updates(
        &mut self,
        request: SlotRangeRequest,
    ) -> Result<impl Stream<Item = Result<LedgerUpdate, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_ledger_updates(proto::SlotRangeRequest::from(request))
            .await?
            .into_inner()
            .map(|msg| TryConvertTo::try_convert(msg?)))
    }
}
