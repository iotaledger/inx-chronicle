// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, Addr, HandleEvent, Report, SpawnActor},
    types::stardust::message::MessageId,
};
use inx::{client::InxClient, tonic::Channel};

use super::{
    config::InxConfig,
    error::InxWorkerError,
    listener::{InxListener, InxListenerError},
    MilestoneState, RequestedMessage,
};
use crate::collector::Collector;

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

#[async_trait]
impl Actor for InxWorker {
    type State = InxClient<Channel>;
    type Error = InxWorkerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        log::info!("Connecting to INX at bind address `{}`.", self.config.connect_url);
        let inx_client = Inx::connect(&self.config).await?;

        log::info!("Connected to INX.");

        cx.spawn_child(InxListener::new(
            self.db.clone(),
            self.config.clone(),
            inx_client.clone(),
        ))
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
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match report.error {
                ActorError::Result(e) => match e {
                    InxListenerError::SubscriptionFailed(_) | InxListenerError::Runtime(_) => {
                        cx.shutdown();
                    }
                    InxListenerError::MissingCollector | InxListenerError::MilestoneGap => {
                        cx.delay(
                            SpawnActor::new(InxListener::new(
                                self.db.clone(),
                                self.config.clone(),
                                inx_client.clone(),
                            )),
                            None,
                        )?;
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

pub struct MessageRequest<Sender: Actor>(MessageId, Addr<Sender>, MilestoneState);
impl<Sender: Actor> MessageRequest<Sender> {
    pub fn new(message_id: MessageId, sender_addr: Addr<Sender>, ms_state: MilestoneState) -> Self {
        Self(message_id, sender_addr, ms_state)
    }
}
pub struct MetadataRequest<Sender: Actor>(MessageId, Addr<Sender>, MilestoneState);
impl<Sender: Actor> MetadataRequest<Sender> {
    pub fn new(message_id: MessageId, sender_addr: Addr<Sender>, ms_state: MilestoneState) -> Self {
        Self(message_id, sender_addr, ms_state)
    }
}

#[async_trait]
impl<Sender: Actor> HandleEvent<MessageRequest<Sender>> for InxWorker
where
    Sender: 'static + HandleEvent<MilestoneState>,
{
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        MessageRequest(message_id, sender_addr, mut ms_state): MessageRequest<Sender>,
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
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
                    .send(RequestedMessage::new(Some(raw), metadata, sender_addr, ms_state))
                    .map_err(|_| InxWorkerError::MissingCollector)?;
            }
            (Err(e), Ok(metadata)) => {
                let metadata = metadata.into_inner();
                // If this isn't a message we care about, don't worry, be happy
                if metadata.milestone_index != ms_state.milestone_index || !metadata.solid {
                    ms_state.process_queue.pop_front();
                    sender_addr.send(ms_state)?;
                } else {
                    log::warn!("Failed to read message: {:?}", e);
                }
            }
            (_, Err(e)) => {
                log::warn!("Failed to read metadata: {:?}", e);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<Sender: Actor> HandleEvent<MetadataRequest<Sender>> for InxWorker
where
    Sender: 'static + HandleEvent<MilestoneState>,
{
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        MetadataRequest(message_id, sender_addr, ms_state): MetadataRequest<Sender>,
        inx_client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if let Ok(metadata) = inx_client
            .read_message_metadata(inx::proto::MessageId {
                id: message_id.0.into(),
            })
            .await
        {
            let metadata = metadata.into_inner();
            cx.addr::<Collector>()
                .await
                .send(RequestedMessage::new(None, metadata, sender_addr, ms_state))
                .map_err(|_| InxWorkerError::MissingCollector)?;
        }
        Ok(())
    }
}
