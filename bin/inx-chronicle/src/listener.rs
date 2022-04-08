// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`InxListener`] subscribes to events from INX and forwards them as an [`InxEvent`] via a Tokio unbounded
//! channel.

use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use chronicle::{
    inx::InxError,
    runtime::{
        actor::{context::ActorContext, envelope::HandleEvent, error::ActorError, handle::Addr, report::Report, Actor},
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

use crate::{broker::Broker, config::Config};

#[derive(Debug, Error)]
pub enum INXListenerError {
    #[error(transparent)]
    Inx(#[from] InxError),
    #[error(transparent)]
    Read(#[from] Status),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[derive(Debug)]
pub struct InxListener;

#[async_trait]
impl Actor for InxListener {
    type Data = InxClient<Channel>;
    type Error = INXListenerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::Data, Self::Error> {
        info!("Connecting to INX...");
        let mut inx_client = if let Some(config) = cx.resource::<Arc<Config>>().await {
            config.inx.build().await
        } else {
            InxClient::connect("http://localhost:9029")
                .await
                .map_err(|_| InxError::TransportFailed)
        }?;

        info!("Connected to INX.");
        let response = inx_client.read_node_status(NoParams {}).await?;
        info!("Node status: {:#?}", response.into_inner());

        let message_stream = inx_client.listen_to_messages(MessageFilter {}).await?.into_inner();
        let milestone_stream = inx_client.listen_to_latest_milestone(NoParams {}).await?.into_inner();

        cx.spawn_actor_supervised::<InxStreamListener<inx::proto::Message>, _>(
            InxStreamListener::new().with_stream(message_stream),
        )
        .await;
        cx.spawn_actor_supervised::<InxStreamListener<inx::proto::Milestone>, _>(
            InxStreamListener::new().with_stream(milestone_stream),
        )
        .await;
        Ok(inx_client)
    }
}

#[async_trait]
impl HandleEvent<Report<InxStreamListener<inx::proto::Message>>> for InxListener {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxStreamListener<inx::proto::Message>>,
        inx_client: &mut Self::Data,
    ) -> Result<(), Self::Error> {
        // TODO: Figure out why `cx.shutdown()` is not working.
        let handle = cx.handle();
        match event {
            Ok(_) => {
                handle.shutdown().await;
            }
            Err(e) => match e.error {
                ActorError::Result(_) | ActorError::Panic => {
                    let message_stream = inx_client.listen_to_messages(MessageFilter {}).await?.into_inner();
                    cx.spawn_actor_supervised::<InxStreamListener<inx::proto::Message>, _>(
                        InxStreamListener::new().with_stream(message_stream),
                    )
                    .await;
                }
                ActorError::Aborted => {
                    handle.shutdown().await;
                }
            },
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Report<InxStreamListener<inx::proto::Milestone>>> for InxListener {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxStreamListener<inx::proto::Milestone>>,
        inx_client: &mut Self::Data,
    ) -> Result<(), Self::Error> {
        // TODO: Figure out why `cx.shutdown()` is not working.
        let handle = cx.handle();
        match event {
            Ok(_) => {
                handle.shutdown().await;
            }
            Err(e) => match e.error {
                ActorError::Result(_) | ActorError::Panic => {
                    let milestone_stream = inx_client.listen_to_latest_milestone(NoParams {}).await?.into_inner();
                    cx.spawn_actor_supervised::<InxStreamListener<inx::proto::Milestone>, _>(
                        InxStreamListener::new().with_stream(milestone_stream),
                    )
                    .await;
                }
                ActorError::Aborted => {
                    handle.shutdown().await;
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

impl<I> Default for InxStreamListener<I> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<E> Actor for InxStreamListener<E>
where
    Broker: HandleEvent<E>,
    E: 'static + Send + Sync + Debug,
{
    type Data = Addr<Broker>;
    type Error = INXListenerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::Data, Self::Error> {
        Ok(cx.link_resource().await?)
    }
}

#[async_trait]
impl<E> HandleEvent<Result<E, Status>> for InxStreamListener<E>
where
    Self: Actor<Data = Addr<Broker>, Error = INXListenerError>,
    Broker: HandleEvent<E>,
    E: 'static + Send + Sync + Debug,
{
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        event: Result<E, Status>,
        broker_handle: &mut Self::Data,
    ) -> Result<(), Self::Error> {
        broker_handle.send(event?).map_err(RuntimeError::SendError)?;
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
