// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`InxListener`] subscribes to events from INX and forwards them via a Tokio unbounded
//! channel.

use std::{fmt::Debug, marker::PhantomData};

use async_trait::async_trait;
use chronicle::runtime::{
    actor::{context::ActorContext, error::ActorError, event::HandleEvent, report::Report, Actor},
    config::ConfigureActor,
};
use inx::{
    client::InxClient,
    proto::{MessageFilter, NoParams},
    tonic::{Channel, Status},
};
use thiserror::Error;

use crate::broker::Broker;

type MessageStream = InxStreamListener<inx::proto::Message>;
type MilestoneStream = InxStreamListener<inx::proto::Milestone>;

#[derive(Debug, Error)]
pub enum InxListenerError {
    #[error("the broker actor is not running")]
    MissingBroker,
    #[error("failed to subscribe to stream: {0}")]
    SubscriptionFailed(#[from] inx::tonic::Status),
    #[error(transparent)]
    Runtime(#[from] crate::RuntimeError),
}

#[derive(Debug)]
pub struct InxListener {
    inx: InxClient<Channel>,
}

impl InxListener {
    // TODO: Should we check for broker actor here too?
    pub fn new(client: InxClient<Channel>) -> Self {
        Self { inx: client }
    }
}

#[async_trait]
impl Actor for InxListener {
    type State = ();
    type Error = InxListenerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let message_stream = self.inx.listen_to_messages(MessageFilter {}).await?.into_inner();

        cx.spawn_child::<MessageStream, _>(InxStreamListener::default().with_stream(message_stream))
            .await;

        let milestone_stream = self.inx.listen_to_latest_milestone(NoParams {}).await?.into_inner();
        cx.spawn_child::<MilestoneStream, _>(InxStreamListener::default().with_stream(milestone_stream))
            .await;

        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<MessageStream>> for InxListener {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<MessageStream>,
        _: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match report.error {
                ActorError::Result(_) => {
                    let message_stream = self.inx.listen_to_messages(MessageFilter {}).await?.into_inner();
                    cx.spawn_child::<MessageStream, _>(InxStreamListener::default().with_stream(message_stream))
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
        _: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match report.error {
                ActorError::Result(_) => {
                    let milestone_stream = self.inx.listen_to_latest_milestone(NoParams {}).await?.into_inner();
                    cx.spawn_child::<MilestoneStream, _>(InxStreamListener::default().with_stream(milestone_stream))
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
    _item: PhantomData<I>,
}

impl<I> Default for InxStreamListener<I> {
    fn default() -> Self {
        Self {
            _item: Default::default(),
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
        cx: &mut ActorContext<Self>,
        event: Result<E, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        cx.addr::<Broker>().await.send(event?)?;
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
