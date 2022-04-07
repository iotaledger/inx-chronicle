// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`InxListener`] subscribes to events from INX and forwards them as an [`InxEvent`] via a Tokio unbounded
//! channel.

use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use chronicle::{
    inx::InxError,
    runtime::{
        actor::{context::ActorContext, envelope::HandleEvent, handle::Act, report::Report, Actor},
        error::RuntimeError,
    },
};
use futures::StreamExt;
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
    INXError(#[from] InxError),
    #[error(transparent)]
    ReadError(#[from] Status),
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
}

#[derive(Debug)]
pub struct InxListener;

#[async_trait]
impl Actor for InxListener {
    type Data = ();
    type Error = INXListenerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::Data, Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
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

        cx.add_resource(inx_client).await;

        cx.spawn_actor_supervised(InxStreamListener::<inx::proto::Message>::new())
            .await?;
        cx.spawn_actor_supervised(InxStreamListener::<inx::proto::Milestone>::new())
            .await?;
        Ok(())
    }
}

#[async_trait]
impl<I> HandleEvent<Report<InxStreamListener<I>>> for InxListener
where
    I: Debug + Send + Sync + 'static,
    Broker: HandleEvent<I>,
    InxStreamListener<I>: Actor,
{
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxStreamListener<I>>,
        _data: &mut Self::Data,
    ) -> Result<(), Self::Error> {
        log::error!("Stream listener exited: {:?}", event);
        let mut retries = 3;
        loop {
            match cx.spawn_actor_supervised(InxStreamListener::<I>::new()).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => {
                    log::error!("{:?}", e);
                    retries -= 1;
                    if retries == 0 {
                        return Err(INXListenerError::RuntimeError(e));
                    }
                }
            }
        }
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
impl Actor for InxStreamListener<inx::proto::Message> {
    type Data = (InxClient<Channel>, Act<Broker>);
    type Error = INXListenerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::Data, Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
        Ok((cx.link_resource().await?, cx.link_resource().await?))
    }

    async fn run(
        &mut self,
        _cx: &mut ActorContext<Self>,
        (inx_client, broker_handle): &mut Self::Data,
    ) -> Result<(), Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
        let mut stream = inx_client.listen_to_messages(MessageFilter {}).await?.into_inner();
        info!("Subscribed to `ListenToMessages`.");
        while let Some(msg) = stream.next().await {
            broker_handle.send(msg?).map_err(RuntimeError::SendError)?;
        }
        Ok(())
    }
}

#[async_trait]
impl Actor for InxStreamListener<inx::proto::Milestone> {
    type Data = (InxClient<Channel>, Act<Broker>);
    type Error = INXListenerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::Data, Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
        Ok((cx.link_resource().await?, cx.link_resource().await?))
    }

    async fn run(
        &mut self,
        _cx: &mut ActorContext<Self>,
        (inx_client, broker_handle): &mut Self::Data,
    ) -> Result<(), Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
        let mut stream = inx_client.listen_to_latest_milestone(NoParams {}).await?.into_inner();
        info!("Subscribed to `ListenToLatestMilestone`.");
        while let Some(msg) = stream.next().await {
            broker_handle.send(msg?).map_err(RuntimeError::SendError)?;
        }
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
