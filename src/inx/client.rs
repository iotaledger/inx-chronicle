// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::stream::{Stream, StreamExt};
use inx::{client::InxClient, proto};
use iota_sdk::types::block::{output::OutputId, Block, BlockId};
use packable::PackableExt;

use super::{
    convert::TryConvertTo,
    ledger::{AcceptedTransaction, LedgerUpdate, UnspentOutput},
    request::SlotRangeRequest,
    responses::{self, BlockMetadata, Commitment, NodeConfiguration, NodeStatus, RootBlocks},
    InxError,
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
        Ok(self.inx.read_node_status(proto::NoParams {}).await?.try_convert()?)
    }

    /// Stream status updates from the node.
    pub async fn get_node_status_updates(
        &mut self,
        request: proto::NodeStatusRequest,
    ) -> Result<impl Stream<Item = Result<NodeStatus, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_node_status(request)
            .await?
            .into_inner()
            .map(|msg| TryConvertTo::try_convert(msg?)))
    }

    /// Get the configuration of the node.
    pub async fn get_node_configuration(&mut self) -> Result<NodeConfiguration, InxError> {
        Ok(self
            .inx
            .read_node_configuration(proto::NoParams {})
            .await?
            .try_convert()?)
    }

    /// Get the active root blocks of the node.
    pub async fn get_active_root_blocks(&mut self) -> Result<RootBlocks, InxError> {
        Ok(self
            .inx
            .read_active_root_blocks(proto::NoParams {})
            .await?
            .try_convert()?)
    }

    /// Get the active root blocks of the node.
    pub async fn get_commitment(&mut self, request: proto::CommitmentRequest) -> Result<Commitment, InxError> {
        Ok(self.inx.read_commitment(request).await?.try_convert()?)
    }

    // /// TODO
    // pub async fn force_commitment_until(&mut self, slot_index: SlotIndex) -> Result<(), InxError> {
    //     self.inx
    //         .force_commit_until(proto::SlotIndex { index: slot_index.0 })
    //         .await?;
    //     Ok(())
    // }

    /// Get a block using a block id.
    pub async fn get_block(&mut self, block_id: BlockId) -> Result<Block, InxError> {
        Ok(self
            .inx
            .read_block(proto::BlockId { id: block_id.to_vec() })
            .await?
            .try_convert()?)
    }

    /// Get a block's metadata using a block id.
    pub async fn get_block_metadata(&mut self, block_id: BlockId) -> Result<BlockMetadata, InxError> {
        Ok(self
            .inx
            .read_block_metadata(proto::BlockId { id: block_id.to_vec() })
            .await?
            .try_convert()?)
    }

    /// Convenience wrapper that gets all blocks.
    pub async fn get_blocks(&mut self) -> Result<impl Stream<Item = Result<responses::Block, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_blocks(proto::NoParams {})
            .await?
            .into_inner()
            .map(|msg| TryConvertTo::try_convert(msg?)))
    }

    /// Convenience wrapper that gets accepted blocks.
    pub async fn get_accepted_blocks(
        &mut self,
    ) -> Result<impl Stream<Item = Result<BlockMetadata, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_accepted_blocks(proto::NoParams {})
            .await?
            .into_inner()
            .map(|msg| TryConvertTo::try_convert(msg?)))
    }

    /// Convenience wrapper that gets confirmed blocks.
    pub async fn get_confirmed_blocks(
        &mut self,
    ) -> Result<impl Stream<Item = Result<BlockMetadata, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_confirmed_blocks(proto::NoParams {})
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

    /// Convenience wrapper that listen to accepted transactions.
    pub async fn get_accepted_transactions(
        &mut self,
    ) -> Result<impl Stream<Item = Result<AcceptedTransaction, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_accepted_transactions(proto::NoParams {})
            .await?
            .into_inner()
            .map(|msg| TryConvertTo::try_convert(msg?)))
    }

    /// Get an output using an output id.
    pub async fn get_output(&mut self, output_id: OutputId) -> Result<responses::Output, InxError> {
        Ok(self
            .inx
            .read_output(proto::OutputId {
                id: output_id.pack_to_vec(),
            })
            .await?
            .try_convert()?)
    }
}
