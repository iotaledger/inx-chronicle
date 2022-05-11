// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use async_trait::async_trait;
use chronicle::runtime::actor::{
    addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, util::SpawnActor, Actor,
};
use inx::{client::InxClient, proto::NoParams, tonic::Channel, NodeStatus};

use super::{
    listener::{InxListener, InxListenerError},
    InxConfig, InxWorkerError,
};
use crate::collector::Collector;

#[derive(Debug)]
pub struct InxWorker {
    config: InxConfig,
}

impl InxWorker {
    pub fn new(config: InxConfig) -> Self {
        Self { config }
    }
}

pub struct Inx;

impl Inx {
    /// Creates an [`InxClient`] by connecting to the endpoint specified in `inx_config`.
    async fn connect(inx_config: &InxConfig) -> Result<InxClient<Channel>, InxWorkerError> {
        let url = url::Url::parse(&inx_config.connect_url)?;

        if url.scheme() != "http" {
            return Err(InxWorkerError::InvalidAddress(inx_config.connect_url.clone()));
        }

        InxClient::connect(inx_config.connect_url.clone())
            .await
            .map_err(InxWorkerError::ConnectionError)
    }
}

#[async_trait]
impl Actor for InxWorker {
    type State = InxClient<Channel>;
    type Error = InxWorkerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Connecting to INX at bind address `{}`.", self.config.connect_url);
        let inx_client = Inx::connect(&self.config).await?;

        log::info!("Connected to INX.");

        cx.spawn_child(InxListener::new(inx_client.clone())).await;

        Ok(inx_client)
    }
}

#[async_trait]
impl HandleEvent<Report<InxListener>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxListener>,
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    InxListenerError::SubscriptionFailed(_) => {
                        cx.shutdown();
                    }
                    InxListenerError::Runtime(_) => {
                        cx.shutdown();
                    }
                    InxListenerError::MissingCollector => {
                        cx.delay(SpawnActor::new(InxListener::new(inx_client.clone())), None)?;
                    }
                },
                ActorError::Panic | ActorError::Aborted => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[cfg(feature = "stardust")]
pub mod stardust {
    use chronicle::dto::MessageId;

    use super::*;
    use crate::collector::stardust::{MilestoneState, RequestedMessage};

    pub(crate) struct NodeStatusRequest<Sender: Actor>(Addr<Sender>);
    impl<Sender: Actor> NodeStatusRequest<Sender> {
        #[allow(dead_code)]
        pub fn new(sender_addr: Addr<Sender>) -> Self {
            Self(sender_addr)
        }
    }
    pub(crate) struct MessageRequest<Sender: Actor>(MessageId, Addr<Sender>, MilestoneState);
    impl<Sender: Actor> MessageRequest<Sender> {
        pub fn new(message_id: MessageId, sender_addr: Addr<Sender>, milestone_state: MilestoneState) -> Self {
            Self(message_id, sender_addr, milestone_state)
        }
    }
    pub(crate) struct MetadataRequest<Sender: Actor>(MessageId, Addr<Sender>, MilestoneState);
    impl<Sender: Actor> MetadataRequest<Sender> {
        pub fn new(message_id: MessageId, sender_addr: Addr<Sender>, milestone_state: MilestoneState) -> Self {
            Self(message_id, sender_addr, milestone_state)
        }
    }
    pub(crate) struct MilestoneRequest(u32);
    impl MilestoneRequest {
        pub fn new(milestone_index: u32) -> Self {
            Self(milestone_index)
        }
    }

    #[async_trait]
    impl<Sender: Actor> HandleEvent<NodeStatusRequest<Sender>> for InxWorker
    where
        Sender: 'static + HandleEvent<NodeStatus>,
    {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            NodeStatusRequest(sender_addr): NodeStatusRequest<Sender>,
            inx_client: &mut Self::State,
        ) -> Result<(), Self::Error> {
            let node_status: NodeStatus = inx_client
                .read_node_status(NoParams {})
                .await?
                .into_inner()
                .try_into()
                .unwrap();

            if !node_status.is_healthy {
                log::warn!("Node is unhealthy.");
            }
            log::info!("Node is at ledger index `{}`.", node_status.ledger_index);

            sender_addr.send(node_status)?;
            Ok(())
        }
    }

    #[async_trait]
    impl<Sender: Actor> HandleEvent<MessageRequest<Sender>> for InxWorker
    where
        Sender: 'static + HandleEvent<MilestoneState>,
    {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            MessageRequest(message_id, sender_addr, mut ms_state): MessageRequest<Sender>,
            inx_client: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match (
                inx_client
                    .read_message(inx::proto::MessageId {
                        id: message_id.0.clone().into(),
                    })
                    .await,
                inx_client
                    .read_message_metadata(inx::proto::MessageId {
                        id: message_id.0.into(),
                    })
                    .await,
            ) {
                (Ok(raw), Ok(metadata)) => {
                    let (raw, metadata) = (raw.into_inner(), metadata.into_inner());
                    cx.addr::<Collector>()
                        .await
                        .send(RequestedMessage::new(Some(raw), metadata, sender_addr, ms_state))
                        .map_err(|_| InxWorkerError::MissingCollector)?;
                }
                (Err(e), Ok(metadata)) => {
                    let metadata = metadata.into_inner();
                    // If this isn't a message we care about, don't worry, be happy
                    if metadata.milestone_index != ms_state.milestone_index || !metadata.solid {
                        ms_state.process_queue.pop_front();
                        sender_addr.send(ms_state)?;
                    } else {
                        log::warn!("Failed to read message: {:?}", e);
                    }
                }
                (_, Err(e)) => {
                    log::warn!("Failed to read metadata: {:?}", e);
                }
            }
            Ok(())
        }
    }

    #[async_trait]
    impl<Sender: Actor> HandleEvent<MetadataRequest<Sender>> for InxWorker
    where
        Sender: 'static + HandleEvent<MilestoneState>,
    {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            MetadataRequest(message_id, sender_addr, ms_state): MetadataRequest<Sender>,
            inx_client: &mut Self::State,
        ) -> Result<(), Self::Error> {
            if let Ok(metadata) = inx_client
                .read_message_metadata(inx::proto::MessageId {
                    id: message_id.0.into(),
                })
                .await
            {
                let metadata = metadata.into_inner();
                cx.addr::<Collector>()
                    .await
                    .send(RequestedMessage::new(None, metadata, sender_addr, ms_state))
                    .map_err(|_| InxWorkerError::MissingCollector)?;
            }
            Ok(())
        }
    }

    #[async_trait]
    impl HandleEvent<MilestoneRequest> for InxWorker {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            MilestoneRequest(milestone_index): MilestoneRequest,
            inx_client: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::trace!("Requesting milestone {}", milestone_index);
            if let Ok(milestone) = inx_client
                .read_milestone(inx::proto::MilestoneRequest {
                    milestone_index,
                    milestone_id: None,
                })
                .await
            {
                // TODO: unwrap
                let milestone: inx::proto::Milestone = milestone.into_inner();

                // Instruct the collector to solidify this milestone.
                cx.addr::<Collector>().await.send(milestone)?;
            } else {
                log::warn!("No milestone response for {}", milestone_index);
            }
            Ok(())
        }
    }
}
