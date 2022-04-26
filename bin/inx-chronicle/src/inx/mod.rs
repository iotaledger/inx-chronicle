// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`InxListener`] subscribes to events from INX and forwards them via a Tokio unbounded
//! channel.

use std::{fmt::Debug, marker::PhantomData, ops::Deref, time::Duration};

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
    broker::Broker,
    collector::{solidifier::Solidifier, stardust::MilestoneState},
    ADDRESS_REGISTRY,
};

type MessageStream = InxStreamListener<inx::proto::Message>;
type MessageMetadataStream = InxStreamListener<inx::proto::MessageMetadata>;
type MilestoneStream = InxStreamListener<inx::proto::Milestone>;

#[derive(Debug, Error)]
pub enum InxListenerError {
    #[error(transparent)]
    Inx(#[from] InxError),
    #[error("the broker actor is not running")]
    MissingBroker,
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
    type Error = InxListenerError;

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
        cx.spawn_actor_supervised::<MessageStream, _>(InxStreamListener::new().with_stream(message_stream))
            .await;

        let metadata_stream = inx_client
            .listen_to_solid_messages(MessageFilter {})
            .await?
            .into_inner();
        cx.spawn_actor_supervised::<MessageMetadataStream, _>(InxStreamListener::new().with_stream(metadata_stream))
            .await;

        let milestone_stream = inx_client.listen_to_latest_milestone(NoParams {}).await?.into_inner();
        cx.spawn_actor_supervised::<MilestoneStream, _>(InxStreamListener::new().with_stream(milestone_stream))
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
                    cx.spawn_actor_supervised::<MessageStream, _>(InxStreamListener::new().with_stream(message_stream))
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
                        .listen_to_solid_messages(MessageFilter {})
                        .await?
                        .into_inner();
                    cx.spawn_actor_supervised::<MessageMetadataStream, _>(
                        InxStreamListener::new().with_stream(message_stream),
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
                        InxStreamListener::new().with_stream(milestone_stream),
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
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match &e.error {
                ActorError::Result(e) => match e.deref() {
                    InxRequesterError::Inx(e) => match e {
                        InxError::ConnectionError(_) => {
                            // TODO
                            let wait_interval = Duration::from_secs(5);
                            log::info!("Retrying INX connection in {} seconds.", wait_interval.as_secs_f32());
                            tokio::time::sleep(wait_interval).await;
                            ADDRESS_REGISTRY
                                .insert(cx.spawn_actor_supervised(InxRequester::new(inx_client.clone())).await)
                                .await;
                        }
                        InxError::InvalidAddress(_) => {
                            cx.shutdown();
                        }
                        InxError::ParsingAddressFailed(_) => {
                            cx.shutdown();
                        }
                        // TODO: This is stupid, but we can't use the ErrorKind enum so :shrug:
                        InxError::TransportFailed(e) => match e.to_string().as_ref() {
                            "transport error" => {
                                ADDRESS_REGISTRY
                                    .insert(cx.spawn_actor_supervised(InxRequester::new(inx_client.clone())).await)
                                    .await;
                            }
                            _ => {
                                cx.shutdown();
                            }
                        },
                    },
                    InxRequesterError::Read(_) => {
                        cx.shutdown();
                    }
                    InxRequesterError::Runtime(_) => {
                        cx.shutdown();
                    }
                    InxRequesterError::MissingBroker => {
                        // If the handle is still closed, push this to the back of the event queue.
                        // Hopefully when it is processed again the handle will have been recreated.
                        if ADDRESS_REGISTRY.get::<Broker>().await.is_none() {
                            cx.delay(event, None).map_err(RuntimeError::SendError)?;
                        } else {
                            ADDRESS_REGISTRY
                                .insert(cx.spawn_actor_supervised(InxRequester::new(inx_client.clone())).await)
                                .await;
                        }
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

pub struct InxStreamListener<I> {
    _item: PhantomData<I>,
}

impl<I> InxStreamListener<I> {
    pub fn new() -> Self {
        Self { _item: PhantomData }
    }
}

#[async_trait]
impl<E> Actor for InxStreamListener<E>
where
    Broker: HandleEvent<E>,
    E: 'static + Send + Sync + Debug,
{
    type State = ();
    type Error = InxListenerError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl<E> HandleEvent<Result<E, Status>> for InxStreamListener<E>
where
    Self: Actor<Error = InxListenerError>,
    Broker: HandleEvent<E>,
    E: 'static + Send + Sync + Debug,
{
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        event: Result<E, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        ADDRESS_REGISTRY
            .get::<Broker>()
            .await
            .send(event?)
            .map_err(RuntimeError::SendError)?;
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

#[derive(Debug, Error)]
pub enum InxRequesterError {
    #[error(transparent)]
    Inx(#[from] InxError),
    #[error("The broker actor is not running")]
    MissingBroker,
    #[error(transparent)]
    Read(#[from] Status),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
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

#[async_trait]
impl Actor for InxRequester {
    type State = ();
    type Error = InxRequesterError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<(MessageId, Addr<Solidifier>, MilestoneState)> for InxRequester {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        (message_id, solidifier, ms_state): (MessageId, Addr<Solidifier>, MilestoneState),
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let message = self
            .inx_client
            .read_message(inx::proto::MessageId {
                id: Vec::from(*message_id.deref()),
            })
            .await?
            .into_inner();
        let metadata = self
            .inx_client
            .read_message_metadata(inx::proto::MessageId {
                id: Vec::from(*message_id.deref()),
            })
            .await?
            .into_inner();

        ADDRESS_REGISTRY
            .get::<Broker>()
            .await
            .send((message, metadata, solidifier, ms_state))
            .map_err(RuntimeError::SendError)?;

        Ok(())
    }
}
