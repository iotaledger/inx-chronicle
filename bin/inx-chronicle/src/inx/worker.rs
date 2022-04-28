// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, ops::Deref};

use async_trait::async_trait;
use chronicle::runtime::actor::{
    addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, Actor,
};
use inx::{client::InxClient, proto::NoParams, tonic::Channel};
use tokio::sync::oneshot;

use super::{
    listener::{InxListener, InxListenerError},
    InxConfig, InxWorkerError,
};
use crate::Broker;

#[derive(Debug)]
pub struct InxWorker {
    config: InxConfig,
    broker_addr: Addr<Broker>,
}

impl InxWorker {
    pub fn new(config: InxConfig, broker_addr: Addr<Broker>) -> Self {
        Self { config, broker_addr }
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
        let mut inx = Inx::connect(&self.config).await?;

        log::info!("Connected to INX.");
        let node_status = inx.read_node_status(NoParams {}).await?.into_inner();

        if !node_status.is_healthy {
            log::warn!("Node is unhealthy.");
        }
        log::info!("Node is at ledger index `{}`.", node_status.ledger_index);

        cx.spawn_actor_supervised(InxListener::new(inx.clone(), self.broker_addr.clone()))
            .await;

        Ok(inx)
    }
}

#[async_trait]
impl HandleEvent<Report<InxListener>> for InxWorker {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxListener>,
        _: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match &e.error {
                ActorError::Result(e) => match e.deref() {
                    InxListenerError::SubscriptionFailed(_) => {
                        cx.shutdown();
                    }
                    InxListenerError::Runtime(_) => {
                        cx.shutdown();
                    }
                    InxListenerError::MissingBroker => {
                        // We bubble up the `MissingBroker` broker error.
                        return Err(Self::Error::MissingBroker);
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

#[cfg(feature = "stardust")]
#[derive(Debug)]
pub struct MessageRequest {
    message_id: bee_message_stardust::MessageId,
    answer_to: Option<oneshot::Sender<bool>>,
}

#[cfg(feature = "stardust")]
#[async_trait]
impl HandleEvent<MessageRequest> for InxWorker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        request: MessageRequest,
        client: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let MessageRequest {
            message_id,
            answer_to: answer,
        } = request;

        let proto_message_id: inx::proto::MessageId = message_id.into();
        if let Ok(message_response) = client.read_message(proto_message_id.clone()).await {
            let raw_message = message_response.into_inner();

            // TODO: Consider own event
            let proto_message = inx::proto::Message {
                message_id: Some(proto_message_id),
                message: Some(raw_message),
            };

            self.broker_addr.send(proto_message)?;

            if let Some(recipient) = answer {
                recipient
                    .send(true)
                    .map_err(|_| InxWorkerError::FailedToAnswerRequest)?;
            }
        } else if let Some(recipient) = answer {
            recipient
                .send(false)
                .map_err(|_| InxWorkerError::FailedToAnswerRequest)?;
        }

        Ok(())
    }
}
