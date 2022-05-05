// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::actor::{
        addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, util::SpawnActor,
        Actor,
    },
};
use inx::{client::InxClient, proto::NoParams, tonic::Channel, NodeStatus};

use super::{
    listener::{InxListener, InxListenerError},
    InxConfig, InxWorkerError,
};
use crate::{
    collector::{solidifier::Solidifier, Collector},
    syncer::Syncer,
};

#[derive(Debug)]
pub struct InxWorker {
    config: InxConfig,
    db: MongoDb,
}

impl InxWorker {
    pub fn new(config: InxConfig, db: MongoDb) -> Self {
        Self { config, db }
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
        let mut inx_client = Inx::connect(&self.config).await?;

        log::info!("Connected to INX.");
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

        cx.spawn_child(InxListener::new(inx_client.clone())).await;

        // Start syncing from the pruning index until the current ledger index of the node.
        let start_index = node_status.pruning_index + 1;
        let end_index = node_status.ledger_index + 1;
        cx.spawn_child::<Syncer, _>(Syncer::new(self.db.clone(), start_index, end_index).with_batch_size(10))
            .await;

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

#[async_trait]
impl HandleEvent<Report<Syncer>> for InxWorker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        _exit: Report<Syncer>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::info!("Syncer finished.");
        Ok(())
    }
}

#[cfg(feature = "stardust")]
pub mod stardust {
    use chronicle::dto::MessageId;

    use super::*;
    use crate::collector::stardust::{MilestoneState, RequestedMessage};

    #[derive(Debug, Clone)]
    pub enum InxRequestType {
        Message(MessageId),
        Metadata(MessageId),
    }

    #[derive(Debug)]
    pub struct InxRequest {
        request_type: InxRequestType,
        solidifier_addr: Addr<Solidifier>,
        ms_state: MilestoneState,
    }

    impl InxRequest {
        pub fn get_message(message_id: MessageId, solidifier_addr: Addr<Solidifier>, ms_state: MilestoneState) -> Self {
            Self {
                request_type: InxRequestType::Message(message_id),
                solidifier_addr,
                ms_state,
            }
        }

        pub fn get_metadata(
            message_id: MessageId,
            solidifier_addr: Addr<Solidifier>,
            ms_state: MilestoneState,
        ) -> Self {
            Self {
                request_type: InxRequestType::Metadata(message_id),
                solidifier_addr,
                ms_state,
            }
        }
    }

    #[async_trait]
    impl HandleEvent<InxRequest> for InxWorker {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            InxRequest {
                request_type,
                solidifier_addr,
                mut ms_state,
            }: InxRequest,
            inx_client: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match request_type {
                InxRequestType::Message(message_id) => {
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
                InxRequestType::Metadata(message_id) => {
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
            }

            Ok(())
        }
    }

    #[async_trait]
    impl HandleEvent<crate::syncer::MilestoneIndex> for InxWorker {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            index: crate::syncer::MilestoneIndex,
            inx_client: &mut Self::State,
        ) -> Result<(), Self::Error> {
            if let Ok(milestone) = inx_client
                .read_milestone(inx::proto::MilestoneRequest {
                    milestone_index: index,
                    milestone_id: None,
                })
                .await
            {
                // TODO: unwrap
                let milestone: inx::proto::Milestone = milestone.into_inner().try_into().unwrap();

                // Instruct the collector to collect this milestone.
                cx.addr::<Collector>().await.send(milestone)?;
            } else {
                log::error!("No milestone response for {index}");
            }

            Ok(())
        }
    }
}
