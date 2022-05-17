// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::actor::{
        addr::Addr,
        context::ActorContext,
        error::ActorError,
        event::HandleEvent,
        report::{ErrorReport, Report},
        util::SpawnActor,
        Actor,
    },
};
use inx::{client::InxClient, proto::NoParams, tonic::Channel, NodeStatus};
use mongodb::error::ErrorKind;

use super::{
    collector::{Collector, CollectorError},
    listener::{InxListener, InxListenerError},
    syncer::{Syncer, NewTargetMilestone},
    InxConfig, InxWorkerError,
};

#[derive(Debug)]
pub struct InxWorker {
    db: MongoDb,
    config: InxConfig,
}

impl InxWorker {
    pub fn new(db: MongoDb, config: InxConfig) -> Self {
        Self { db, config }
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

pub struct InxWorkerState {
    inx_client: InxClient<Channel>,
    syncer_started: bool,
}

#[async_trait]
impl Actor for InxWorker {
    type State = InxWorkerState;
    type Error = InxWorkerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Connecting to INX at bind address `{}`.", self.config.connect_url);
        let inx_client = Inx::connect(&self.config).await?;
        log::info!("Connected to INX.");

        cx.spawn_child(Collector::new(self.db.clone(), self.config.collector.clone()))
            .await;
        cx.spawn_child(InxListener::new(inx_client.clone())).await;
        cx.spawn_child(Syncer::new(self.db.clone(), self.config.syncer.clone()))
            .await;

        Ok(InxWorkerState { inx_client, syncer_started: false } )
    }
}

#[async_trait]
impl HandleEvent<Report<Collector>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Collector>,
        _: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    CollectorError::MongoDb(e) => match e.kind.as_ref() {
                        // Only a few possible errors we could potentially recover from
                        ErrorKind::Io(_) | ErrorKind::ServerSelection { message: _, .. } => {
                            cx.spawn_child(Collector::new(self.db.clone(), self.config.collector.clone()))
                                .await;
                        }
                        _ => {
                            cx.shutdown();
                        }
                    },
                    _ => {
                        cx.shutdown();
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
impl HandleEvent<Report<InxListener>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxListener>,
        state: &mut Self::State,
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
                        cx.delay(SpawnActor::new(InxListener::new(state.inx_client.clone())), None)?;
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
        cx: &mut ActorContext<Self>,
        report: Report<Syncer>,
        _: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match report {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => {
                let ErrorReport {
                    error, internal_state, ..
                } = report;

                match error {
                    ActorError::Result(e) => {
                        log::error!("Syncer exited with error: {}", e);
                        // Panic: a previous Syncer instance always has an internal state.
                        cx.spawn_child(
                            Syncer::new(self.db.clone(), self.config.syncer.clone())
                                .with_internal_state(internal_state.unwrap()),
                        )
                        .await;
                    }
                    ActorError::Panic | ActorError::Aborted => {
                        cx.shutdown();
                    }
                }
                cx.shutdown();
            }
        }
        Ok(())
    }
}

pub struct NewMilestone(pub(crate) u32);

#[async_trait]
impl HandleEvent<NewMilestone> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        NewMilestone(milestone_index): NewMilestone,
        state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if !state.syncer_started {
            cx.addr::<Syncer>().await.send(NewTargetMilestone(milestone_index.max(1) - 1))?;
            state.syncer_started = true;
        }
        Ok(())
    }
}

#[cfg(feature = "stardust")]
pub mod stardust {
    use std::time::Instant;

    use bee_message_stardust::payload::milestone::MilestoneIndex;
    use chronicle::dto::MessageId;

    use super::*;
    use crate::inx::collector::{
        solidifier::Solidifier,
        stardust::{MilestoneState, RequestedMessage, RequestedMilestone},
    };

    #[derive(Debug)]
    pub enum InxRequest {
        Message(MessageId, Addr<Solidifier>, MilestoneState),
        Metadata(MessageId, Addr<Solidifier>, MilestoneState),
    }

    impl InxRequest {
        pub fn message(message_id: MessageId, callback_addr: Addr<Solidifier>, ms_state: MilestoneState) -> Self {
            Self::Message(message_id, callback_addr, ms_state)
        }

        pub fn metadata(message_id: MessageId, callback_addr: Addr<Solidifier>, ms_state: MilestoneState) -> Self {
            Self::Metadata(message_id, callback_addr, ms_state)
        }
    }

    #[derive(Debug)]
    pub struct NodeStatusRequest {
        pub syncer_addr: Addr<Syncer>,
    }

    #[async_trait]
    impl HandleEvent<NodeStatusRequest> for InxWorker {
        async fn handle_event(
            &mut self,
            _: &mut ActorContext<Self>,
            NodeStatusRequest { syncer_addr }: NodeStatusRequest,
            state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            let now = Instant::now();
            let node_status: NodeStatus = state.inx_client
                .read_node_status(NoParams {})
                .await?
                .into_inner()
                .try_into()
                .unwrap();
            log::debug!("Requested node status. Took {}s.", now.elapsed().as_secs_f32());

            if !node_status.is_healthy {
                log::warn!("Node is unhealthy.");
            }
            log::info!("Node is at ledger index `{}`.", node_status.ledger_index);

            syncer_addr.send(node_status)?;

            Ok(())
        }
    }

    #[derive(Debug)]
    pub struct MilestoneRequest {
        pub milestone_index: MilestoneIndex,
        pub syncer_addr: Addr<Syncer>,
    }

    #[async_trait]
    impl HandleEvent<MilestoneRequest> for InxWorker {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            MilestoneRequest {
                milestone_index,
                syncer_addr,
            }: MilestoneRequest,
            state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            let now = Instant::now();
            if let Ok(milestone) = state.inx_client
                .read_milestone(inx::proto::MilestoneRequest {
                    milestone_index: *milestone_index,
                    milestone_id: None,
                })
                .await
            {
                log::debug!(
                    "Requested milestone '{}'. Took {}s.",
                    *milestone_index,
                    now.elapsed().as_secs_f32()
                );
                let milestone: inx::proto::Milestone = milestone.into_inner();

                // Instruct the collector to solidify this milestone.
                cx.addr::<Collector>()
                    .await
                    .send(RequestedMilestone::new(milestone, syncer_addr))?;
            } else {
                log::warn!("No milestone response for {}", *milestone_index);
            }
            Ok(())
        }
    }

    #[async_trait]
    impl HandleEvent<InxRequest> for InxWorker {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            inx_request: InxRequest,
            state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match inx_request {
                InxRequest::Message(message_id, solidifier_addr, mut ms_state) => {
                    let now = Instant::now();
                    match (
                        state.inx_client
                            .read_message(inx::proto::MessageId {
                                id: message_id.0.clone().into(),
                            })
                            .await,
                        state.inx_client
                            .read_message_metadata(inx::proto::MessageId {
                                id: message_id.0.into(),
                            })
                            .await,
                    ) {
                        (Ok(raw), Ok(metadata)) => {
                            log::debug!("Request message/metadata. Took {}s.", now.elapsed().as_secs_f32());

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
                    let now = Instant::now();
                    if let Ok(metadata) = state.inx_client
                        .read_message_metadata(inx::proto::MessageId {
                            id: message_id.0.into(),
                        })
                        .await
                    {
                        log::debug!("Requested metadata. Took {}s.", now.elapsed().as_secs_f32());

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
}
