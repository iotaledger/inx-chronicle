// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`InxListener`] subscribes to events from INX and forwards them via a Tokio unbounded
//! channel.

use std::{fmt::Debug, marker::PhantomData};

use async_trait::async_trait;
use chronicle::{
    inx::{InxConfig, InxError},
    runtime::{
        actor::{addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, Actor},
        config::ConfigureActor,
        error::RuntimeError,
    },
};
use inx::{
    client::InxClient,
    proto::{MessageFilter, NoParams},
    Channel, Status,
};
use log::info;
use thiserror::Error;

use crate::broker::Broker;

type MessageStream = InxStreamListener<inx::proto::Message>;
type MilestoneStream = InxStreamListener<inx::proto::Milestone>;

#[derive(Debug, Error)]
pub enum InxListenerError {
    #[error("The broker actor is not running")]
    MissingBroker,
    #[error(transparent)]
    Inx(#[from] InxError),
    #[error(transparent)]
    Read(#[from] Status),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[derive(Debug)]
pub struct InxListener {
    config: InxConfig,
    broker_addr: Addr<Broker>,
}

impl InxListener {
    pub fn new(config: InxConfig, broker_addr: Addr<Broker>) -> Self {
        Self { config, broker_addr }
    }
}

#[async_trait]
impl Actor for InxListener {
    type State = InxClient<Channel>;
    type Error = InxListenerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        info!("Connecting to INX...");
        let mut inx_client = self.config.build().await?;

        info!("Connected to INX.");
        let response = inx_client.read_node_status(NoParams {}).await?;
        info!("Node status: {:#?}", response.into_inner());

        let message_stream = inx_client.listen_to_messages(MessageFilter {}).await?.into_inner();
        cx.spawn_actor_supervised::<MessageStream, _>(
            InxStreamListener::new(self.broker_addr.clone())?.with_stream(message_stream),
        )
        .await;

        let milestone_stream = inx_client.listen_to_latest_milestone(NoParams {}).await?.into_inner();
        cx.spawn_actor_supervised::<MilestoneStream, _>(
            InxStreamListener::new(self.broker_addr.clone())?.with_stream(milestone_stream),
        )
        .await;

        Ok(inx_client)
    }
}

#[async_trait]
impl HandleEvent<Report<MessageStream>> for InxListener {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MessageStream>,
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match e.error {
                ActorError::Result(_) => {
                    let message_stream = inx_client.listen_to_messages(MessageFilter {}).await?.into_inner();
                    cx.spawn_actor_supervised::<MessageStream, _>(
                        InxStreamListener::new(self.broker_addr.clone())?.with_stream(message_stream),
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
impl HandleEvent<Report<MilestoneStream>> for InxListener {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MilestoneStream>,
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match e.error {
                ActorError::Result(_) => {
                    let milestone_stream = inx_client.listen_to_latest_milestone(NoParams {}).await?.into_inner();
                    cx.spawn_actor_supervised::<MilestoneStream, _>(
                        InxStreamListener::new(self.broker_addr.clone())?.with_stream(milestone_stream),
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

pub struct InxStreamListener<I> {
    broker_addr: Addr<Broker>,
    _item: PhantomData<I>,
}

impl<I> InxStreamListener<I> {
    pub fn new(broker_addr: Addr<Broker>) -> Result<Self, InxListenerError> {
        if broker_addr.is_closed() {
            Err(InxListenerError::MissingBroker)
        } else {
            Ok(Self {
                broker_addr,
                _item: PhantomData,
            })
        }
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
        self.broker_addr.send(event?).map_err(RuntimeError::SendError)?;
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
