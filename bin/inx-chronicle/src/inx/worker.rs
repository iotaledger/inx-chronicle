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
use crate::collector::{solidifier::Solidifier, Collector};

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

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Connecting to INX at bind address `{}`.", self.config.connect_url);
        let inx_client = Inx::connect(&self.config).await?;

        log::info!("Connected to INX.");

        // TODO: turn this back on!
        // cx.spawn_child(InxListener::new(inx_client.clone())).await;

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
    use bee_message_stardust::payload::milestone::MilestoneIndex;
    use chronicle::dto::MessageId;

    use super::*;
    use crate::{
        collector::stardust::{MilestoneState, RequestedMessage},
        launcher::Launcher,
    };

    #[derive(Debug)]
    pub enum InxRequest {
        NodeStatus,
        Message(MessageId, Addr<Solidifier>, MilestoneState),
        Metadata(MessageId, Addr<Solidifier>, MilestoneState),
        Milestone(MilestoneIndex),
    }

    impl InxRequest {
        pub fn message(message_id: MessageId, solidifier_addr: Addr<Solidifier>, ms_state: MilestoneState) -> Self {
            Self::Message(message_id, solidifier_addr, ms_state)
        }

        pub fn metadata(message_id: MessageId, solidifier_addr: Addr<Solidifier>, ms_state: MilestoneState) -> Self {
            Self::Metadata(message_id, solidifier_addr, ms_state)
        }

        pub fn milestone(milestone_index: MilestoneIndex) -> Self {
            Self::Milestone(milestone_index)
        }
    }

    #[async_trait]
    impl HandleEvent<InxRequest> for InxWorker {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            inx_request: InxRequest,
            inx_client: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match inx_request {
                InxRequest::NodeStatus => {
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

                    cx.addr::<Launcher>().await.send(node_status)?;
                }
                InxRequest::Message(message_id, solidifier_addr, mut ms_state) => {
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
                                .send(RequestedMessage::new(Some(raw), metadata, solidifier_addr, ms_state))
                                .map_err(|_| InxWorkerError::MissingCollector)?;
                        }
                        (Err(e), Ok(metadata)) => {
                            let metadata = metadata.into_inner();
                            // If this isn't a message we care about, don't worry, be happy
                            if metadata.milestone_index != ms_state.milestone_index || !metadata.solid {
                                ms_state.process_queue.pop_front();
                                solidifier_addr.send(ms_state)?;
                            } else {
                                log::warn!("Failed to read message: {:?}", e);
                            }
                        }
                        (_, Err(e)) => {
                            log::warn!("Failed to read metadata: {:?}", e);
                        }
                    }
                }
                InxRequest::Metadata(message_id, solidifier_addr, ms_state) => {
                    if let Ok(metadata) = inx_client
                        .read_message_metadata(inx::proto::MessageId {
                            id: message_id.0.into(),
                        })
                        .await
                    {
                        let metadata = metadata.into_inner();
                        cx.addr::<Collector>()
                            .await
                            .send(RequestedMessage::new(None, metadata, solidifier_addr, ms_state))
                            .map_err(|_| InxWorkerError::MissingCollector)?;
                    }
                }
                InxRequest::Milestone(milestone_index) => {
                    log::trace!("Requesting milestone {}", *milestone_index);
                    if let Ok(milestone) = inx_client
                        .read_milestone(inx::proto::MilestoneRequest {
                            milestone_index: *milestone_index,
                            milestone_id: None,
                        })
                        .await
                    {
                        // TODO: unwrap
                        let milestone: inx::proto::Milestone = milestone.into_inner();

                        // Instruct the collector to solidify this milestone.
                        cx.addr::<Collector>().await.send(milestone)?;
                    } else {
                        log::warn!("No milestone response for {}", *milestone_index);
                    }
                }
            }

            Ok(())
        }
    }
}
