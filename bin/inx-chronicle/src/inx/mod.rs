// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`InxWorker`] subscribes to events from INX and forwards them via a Tokio unbounded
//! channel.

use std::{fmt::Debug, marker::PhantomData, ops::Deref};

use async_trait::async_trait;
use chronicle::{
    inx::{InxConfig, InxError},
    runtime::{
        actor::{addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, Actor},
        config::ConfigureActor,
        error::RuntimeError,
    },
    stardust::MessageId,
};
use inx::{
    client::InxClient,
    proto::{MessageFilter, NoParams},
    tonic::{Channel, Status},
};
use thiserror::Error;

use crate::{
    collector::{
        solidifier::Solidifier,
        stardust::{MilestoneState, RequestedMessage},
        Collector,
    },
    ADDRESS_REGISTRY,
};

type MessageStream = InxStreamListener<inx::proto::Message>;
type MessageMetadataStream = InxStreamListener<inx::proto::MessageMetadata>;
type MilestoneStream = InxStreamListener<inx::proto::Milestone>;

#[derive(Debug, Error)]
pub enum InxWorkerError {
    #[error(transparent)]
    Inx(#[from] InxError),
    #[error("the collector is not running")]
    MissingCollector,
    #[error(transparent)]
    Read(#[from] Status),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[derive(Debug)]
pub struct InxWorker {
    config: InxConfig,
}

impl InxWorker {
    pub fn new(config: InxConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Actor for InxWorker {
    type State = InxClient<Channel>;
    type Error = InxWorkerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Connecting to INX at bind address `{}`.", self.config.address);
        let mut inx_client = self.config.build().await?;

        log::info!("Connected to INX.");
        let node_status = inx_client.read_node_status(NoParams {}).await?.into_inner();

        if !node_status.is_healthy {
            log::warn!("Node is unhealthy.");
        }
        log::info!("Node is at ledger index `{}`.", node_status.ledger_index);

        let message_stream = inx_client.listen_to_messages(MessageFilter {}).await?.into_inner();
        cx.spawn_actor_supervised::<MessageStream, _>(InxStreamListener::default().with_stream(message_stream))
            .await;

        let metadata_stream = inx_client
            .listen_to_referenced_messages(MessageFilter {})
            .await?
            .into_inner();
        cx.spawn_actor_supervised::<MessageMetadataStream, _>(
            InxStreamListener::default().with_stream(metadata_stream),
        )
        .await;

        let milestone_stream = inx_client.listen_to_latest_milestone(NoParams {}).await?.into_inner();
        cx.spawn_actor_supervised::<MilestoneStream, _>(InxStreamListener::default().with_stream(milestone_stream))
            .await;

        ADDRESS_REGISTRY
            .insert(cx.spawn_actor_supervised(InxRequester::new(inx_client.clone())).await)
            .await;

        Ok(inx_client)
    }
}

#[async_trait]
impl HandleEvent<Report<MessageStream>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MessageStream>,
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let message_stream = inx_client.listen_to_messages(MessageFilter {}).await?.into_inner();
                    cx.spawn_actor_supervised::<MessageStream, _>(
                        InxStreamListener::default().with_stream(message_stream),
                    )
                    .await;
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<MessageMetadataStream>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MessageMetadataStream>,
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let message_stream = inx_client
                        .listen_to_referenced_messages(MessageFilter {})
                        .await?
                        .into_inner();
                    cx.spawn_actor_supervised::<MessageMetadataStream, _>(
                        InxStreamListener::default().with_stream(message_stream),
                    )
                    .await;
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<MilestoneStream>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MilestoneStream>,
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let milestone_stream = inx_client.listen_to_latest_milestone(NoParams {}).await?.into_inner();
                    cx.spawn_actor_supervised::<MilestoneStream, _>(
                        InxStreamListener::default().with_stream(milestone_stream),
                    )
                    .await;
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<InxRequester>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxRequester>,
        _inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(e) => {
                    return Err(e);
                }
                ActorError::Panic | ActorError::Aborted => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

pub struct InxStreamListener<I> {
    _item: PhantomData<I>,
}

impl<I> Default for InxStreamListener<I> {
    fn default() -> Self {
        Self { _item: PhantomData }
    }
}

#[async_trait]
impl<E> Actor for InxStreamListener<E>
where
    Collector: HandleEvent<E>,
    E: 'static + Send + Sync + Debug,
{
    type State = ();
    type Error = InxWorkerError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl<E> HandleEvent<Result<E, Status>> for InxStreamListener<E>
where
    Self: Actor<Error = InxWorkerError>,
    Collector: HandleEvent<E>,
    E: 'static + Send + Sync + Debug,
{
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        event: Result<E, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        ADDRESS_REGISTRY
            .get::<Collector>()
            .await
            .send(event?)
            .map_err(|_| InxWorkerError::MissingCollector)?;
        Ok(())
    }
}

impl<I> Debug for InxStreamListener<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InxStreamListener")
            .field("item", &std::any::type_name::<I>())
            .finish()
    }
}

#[derive(Debug)]
pub struct InxRequester {
    inx_client: InxClient<Channel>,
}

impl InxRequester {
    pub fn new(inx_client: InxClient<Channel>) -> Self {
        Self { inx_client }
    }
}

#[derive(Debug, Copy, Clone)]
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

    pub fn get_metadata(message_id: MessageId, solidifier_addr: Addr<Solidifier>, ms_state: MilestoneState) -> Self {
        Self {
            request_type: InxRequestType::Metadata(message_id),
            solidifier_addr,
            ms_state,
        }
    }
}

#[async_trait]
impl Actor for InxRequester {
    type State = ();
    type Error = InxWorkerError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<InxRequest> for InxRequester {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        InxRequest {
            request_type,
            solidifier_addr,
            mut ms_state,
        }: InxRequest,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match request_type {
            InxRequestType::Message(message_id) => {
                match (
                    self.inx_client
                        .read_message(inx::proto::MessageId {
                            id: Vec::from(*message_id.deref()),
                        })
                        .await,
                    self.inx_client
                        .read_message_metadata(inx::proto::MessageId {
                            id: Vec::from(*message_id.deref()),
                        })
                        .await,
                ) {
                    (Ok(raw), Ok(metadata)) => {
                        let (raw, metadata) = (raw.into_inner(), metadata.into_inner());
                        ADDRESS_REGISTRY
                            .get::<Collector>()
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
                if let Ok(metadata) = self
                    .inx_client
                    .read_message_metadata(inx::proto::MessageId {
                        id: Vec::from(*message_id.deref()),
                    })
                    .await
                {
                    let metadata = metadata.into_inner();
                    ADDRESS_REGISTRY
                        .get::<Collector>()
                        .await
                        .send(RequestedMessage::new(None, metadata, solidifier_addr, ms_state))
                        .map_err(|_| InxWorkerError::MissingCollector)?;
                }
            }
        }

        Ok(())
    }
}
