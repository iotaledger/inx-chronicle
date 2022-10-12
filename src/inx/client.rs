// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::stream::{Stream, StreamExt};
use inx::{proto, client::InxClient};

use super::{InxError, MilestoneRangeRequest, LedgerUpdateMessage, NodeStatusMessage};

/// An INX client connection.
#[derive(Clone, Debug)]
pub struct Inx {
    inx: InxClient<inx::tonic::transport::Channel>,
}

fn unpack_proto_msg<Proto, T>(msg: Result<Proto, tonic::Status>) -> Result<T, InxError>
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

    // /// Listens to confirmed milestones in the range of
    // pub async fn listen_to_confirmed_milestones(
    //     &mut self,
    //     request: MilestoneRangeRequest,
    // ) -> Result<impl Stream<Item = Result<crate::MilestoneAndProtocolParameters, Error>>, Error> {
    //     Ok(self
    //         .inx
    //         .listen_to_confirmed_milestones(proto::MilestoneRangeRequest::from(request))
    //         .await?
    //         .into_inner()
    //         .map(unpack_proto_msg))
    // }

    pub async fn listen_to_ledger_updates(
        &mut self,
        request: MilestoneRangeRequest,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateMessage, InxError>>, InxError> {

        Ok(self
            .inx
            .listen_to_ledger_updates(inx::proto::MilestoneRangeRequest::from(request))
            .await?
            .into_inner().map(unpack_proto_msg))
    }

    pub async fn read_node_status(&mut self) -> Result<NodeStatusMessage, InxError> {
        NodeStatusMessage::try_from(self.inx.read_node_status(proto::NoParams {}).await?.into_inner())
    }

    // pub async fn read_node_configuration(&mut self) -> Result<NodeConfiguration, Error> {
    //     NodeConfiguration::try_from(self.inx.read_node_configuration(proto::NoParams {}).await?.into_inner())
    //         .map_err(Error::InxError)
    // }

    // pub async fn read_unspent_outputs(
    //     &mut self,
    // ) -> Result<impl Stream<Item = Result<crate::UnspentOutput, Error>>, Error> {
    //     Ok(self
    //         .inx
    //         .read_unspent_outputs(proto::NoParams {})
    //         .await?
    //         .into_inner()
    //         .map(unpack_proto_msg))
    // }

    // pub async fn read_protocol_parameters(
    //     &mut self,
    //     request: MilestoneRequest,
    // ) -> Result<crate::ProtocolParameters, Error> {
    //     Ok(self
    //         .inx
    //         .read_protocol_parameters(proto::MilestoneRequest::from(request))
    //         .await?
    //         .into_inner()
    //         .into())
    // }

    // /// Reads the past cone of a milestone specified by a [`MilestoneRequest`].
    // pub async fn read_milestone_cone(
    //     &mut self,
    //     request: MilestoneRequest,
    // ) -> Result<impl Stream<Item = Result<crate::BlockWithMetadata, Error>>, Error> {
    //     Ok(self
    //         .inx
    //         .read_milestone_cone(proto::MilestoneRequest::from(request))
    //         .await?
    //         .into_inner()
    //         .map(unpack_proto_msg))
    // }

    // pub async fn read_milestone(&mut self, request: MilestoneRequest) -> Result<Milestone, Error> {
    //     Milestone::try_from(
    //         self.inx
    //             .read_milestone(proto::MilestoneRequest::from(request))
    //             .await?
    //             .into_inner(),
    //     )
    //     .map_err(Error::InxError)
    // }
}
