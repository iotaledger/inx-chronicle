// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`InxListener`] subscribes to events from INX and forwards them via a Tokio unbounded
//! channel.

use std::{fmt::Debug, marker::PhantomData};

use async_trait::async_trait;
use chronicle::runtime::{Actor, ActorContext, ActorError, ConfigureActor, HandleEvent, Report};
use inx::{
    client::InxClient,
    proto::NoParams,
    tonic::{Channel, Status},
};
use thiserror::Error;

use crate::collector::Collector;

type BlockStream = InxStreamListener<inx::proto::Block>;
type BlockMetadataStream = InxStreamListener<inx::proto::BlockMetadata>;
type MilestoneStream = InxStreamListener<inx::proto::Milestone>;

#[derive(Debug, Error)]
pub enum StardustInxListenerError {
    #[error("the collector is not running")]
    MissingCollector,
    #[error("failed to subscribe to stream: {0}")]
    SubscriptionFailed(#[from] inx::tonic::Status),
    #[error(transparent)]
    Runtime(#[from] crate::RuntimeError),
}

#[derive(Debug)]
pub struct InxListener {
    inx_client: InxClient<Channel>,
}

impl InxListener {
    // TODO: Should we check for broker actor here too?
    pub fn new(inx_client: InxClient<Channel>) -> Self {
        Self { inx_client }
    }
}

#[async_trait]
impl Actor for InxListener {
    type State = ();
    type Error = StardustInxListenerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let block_stream = self.inx_client.listen_to_blocks(NoParams {}).await?.into_inner();
        cx.spawn_child::<BlockStream, _>(
            InxStreamListener::default()
                .with_stream(block_stream)
                .with_registration(false),
        )
        .await;

        let metadata_stream = self
            .inx_client
            .listen_to_referenced_blocks(NoParams {})
            .await?
            .into_inner();
        cx.spawn_child::<BlockMetadataStream, _>(
            InxStreamListener::default()
                .with_stream(metadata_stream)
                .with_registration(false),
        )
        .await;

        let milestone_stream = self
            .inx_client
            .listen_to_confirmed_milestones(inx::proto::MilestoneRangeRequest::from(..))
            .await?
            .into_inner();
        cx.spawn_child::<MilestoneStream, _>(
            InxStreamListener::default()
                .with_stream(milestone_stream)
                .with_registration(false),
        )
        .await;

        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<BlockStream>> for InxListener {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<BlockStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let block_stream = self.inx_client.listen_to_blocks(NoParams {}).await?.into_inner();
                    cx.spawn_child::<BlockStream, _>(
                        InxStreamListener::default()
                            .with_stream(block_stream)
                            .with_registration(false),
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
impl HandleEvent<Report<BlockMetadataStream>> for InxListener {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<BlockMetadataStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let block_stream = self
                        .inx_client
                        .listen_to_referenced_blocks(NoParams {})
                        .await?
                        .into_inner();
                    cx.spawn_child::<BlockMetadataStream, _>(
                        InxStreamListener::default()
                            .with_stream(block_stream)
                            .with_registration(false),
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
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    let milestone_stream = self
                        .inx_client
                        .listen_to_confirmed_milestones(inx::proto::MilestoneRangeRequest::from(..))
                        .await?
                        .into_inner();
                    cx.spawn_child::<MilestoneStream, _>(
                        InxStreamListener::default()
                            .with_stream(milestone_stream)
                            .with_registration(false),
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
    // This funky phantom fn pointer is so that the type impls Send + Sync
    _item: PhantomData<fn(I) -> I>,
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
    E: 'static + Send,
{
    type State = ();
    type Error = StardustInxListenerError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl<E> HandleEvent<Result<E, Status>> for InxStreamListener<E>
where
    Self: Actor<Error = StardustInxListenerError>,
    Collector: HandleEvent<E>,
    E: 'static + Send,
{
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Result<E, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        cx.addr::<Collector>()
            .await
            .send(event?)
            .map_err(|_| StardustInxListenerError::MissingCollector)?;
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
