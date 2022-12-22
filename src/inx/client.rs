// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::stream::{Stream, StreamExt};
use inx::{client::InxClient, proto};

use super::{
    block::BlockWithMetadataMessage,
    ledger::UnspentOutputMessage,
    milestone::{MilestoneAndProtocolParametersMessage, MilestoneMessage},
    node::NodeConfigurationMessage,
    request::MilestoneRequest,
    InxError, LedgerUpdateMessage, MilestoneRangeRequest, NodeStatusMessage, RawProtocolParametersMessage,
};

/// An INX client connection.
#[derive(Clone, Debug)]
pub struct Inx {
    pub(crate) inx: InxClient<inx::tonic::transport::Channel>,
}

// TODO: Remove duplicate
pub(crate) fn unpack_proto_msg<Proto, T>(msg: Result<Proto, tonic::Status>) -> Result<T, InxError>
where
    T: TryFrom<Proto, Error = InxError>,
{
    let inner = msg.map_err(InxError::StatusCode)?;
    T::try_from(inner)
}

impl Inx {
    /// Connect to the INX interface of a node.
    pub async fn connect(address: String) -> Result<Self, InxError> {
        Ok(Self {
            inx: InxClient::connect(address).await?,
        })
    }

    /// Convenience wrapper that listen to ledger updates as a stream of
    /// [`MilestoneAndProtocolParametersMessages`](MilestoneAndProtocolParametersMessage).
    pub async fn listen_to_confirmed_milestones(
        &mut self,
        request: MilestoneRangeRequest,
    ) -> Result<impl Stream<Item = Result<MilestoneAndProtocolParametersMessage, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_confirmed_milestones(proto::MilestoneRangeRequest::from(request))
            .await?
            .into_inner()
            .map(unpack_proto_msg))
    }

    /// Convenience wrapper that listen to ledger updates as a stream of [`NodeStatusMessages`](NodeStatusMessage).
    pub async fn listen_to_ledger_updates(
        &mut self,
        request: MilestoneRangeRequest,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateMessage, InxError>>, InxError> {
        Ok(self
            .inx
            .listen_to_ledger_updates(inx::proto::MilestoneRangeRequest::from(request))
            .await?
            .into_inner()
            .map(unpack_proto_msg))
    }

    /// Convenience wrapper that reads the status of the node into a [`NodeStatusMessage`].
    pub async fn read_node_status(&mut self) -> Result<NodeStatusMessage, InxError> {
        NodeStatusMessage::try_from(self.inx.read_node_status(proto::NoParams {}).await?.into_inner())
    }

    /// Convenience wrapper that reads the configuration of the node into a [`NodeConfigurationMessage`].
    pub async fn read_node_configuration(&mut self) -> Result<NodeConfigurationMessage, InxError> {
        NodeConfigurationMessage::try_from(self.inx.read_node_configuration(proto::NoParams {}).await?.into_inner())
    }

    /// Convenience wrapper that reads the current unspent outputs into an [`UnspentOutputMessage`].
    pub async fn read_unspent_outputs(
        &mut self,
    ) -> Result<impl Stream<Item = Result<UnspentOutputMessage, InxError>>, InxError> {
        Ok(self
            .inx
            .read_unspent_outputs(proto::NoParams {})
            .await?
            .into_inner()
            .map(unpack_proto_msg))
    }

    /// Convenience wrapper that reads the protocol parameters for a given milestone into a
    /// [`RawProtocolParametersMessage`].
    pub async fn read_protocol_parameters(
        &mut self,
        request: MilestoneRequest,
    ) -> Result<RawProtocolParametersMessage, InxError> {
        Ok(self
            .inx
            .read_protocol_parameters(proto::MilestoneRequest::from(request))
            .await?
            .into_inner()
            .into())
    }

    /// Convenience wrapper that reads the milestone cone for a given milestone into
    /// [`BlockWithMetadataMessages`](BlockWithMetadataMessage).
    pub async fn read_milestone_cone(
        &mut self,
        request: MilestoneRequest,
    ) -> Result<impl Stream<Item = Result<BlockWithMetadataMessage, InxError>>, InxError> {
        Ok(self
            .inx
            .read_milestone_cone(proto::MilestoneRequest::from(request))
            .await?
            .into_inner()
            .map(unpack_proto_msg))
    }

    /// Convenience wrapper that reads the information for a given milestone.
    pub async fn read_milestone(&mut self, request: MilestoneRequest) -> Result<MilestoneMessage, InxError> {
        MilestoneMessage::try_from(
            self.inx
                .read_milestone(proto::MilestoneRequest::from(request))
                .await?
                .into_inner(),
        )
    }
}
