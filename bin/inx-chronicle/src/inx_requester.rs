// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The [`InxRequester`] requests data from INX and forwards them via a Tokio unbounded
//! channel.

use std::fmt::Debug;

use async_trait::async_trait;
use chronicle::{
    inx::{InxConfig, InxError},
    runtime::{
        actor::{addr::Addr, context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
    stardust::MessageId,
};
use inx::{
    client::InxClient,
    proto::NoParams,
    tonic::{Channel, Status},
};
use log::info;
use thiserror::Error;
use tokio::sync::oneshot;

use crate::broker::Broker;

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
    config: InxConfig,
    broker_addr: Addr<Broker>,
}

impl InxRequester {
    pub fn new(config: InxConfig, broker_addr: Addr<Broker>) -> Self {
        Self { config, broker_addr }
    }
}

#[async_trait]
impl Actor for InxRequester {
    type State = InxClient<Channel>;
    type Error = InxRequesterError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        info!("Connecting to INX...");
        let mut inx_client = self.config.build().await?;

        info!("Connected to INX.");
        let response = inx_client.read_node_status(NoParams {}).await?;
        info!("Node status: {:#?}", response.into_inner());

        Ok(inx_client)
    }
}

#[async_trait]
impl HandleEvent<(MessageId, oneshot::Sender<bool>)> for InxRequester {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        (message_id, channel): (MessageId, oneshot::Sender<bool>),
        state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        let message: Option<inx::proto::Message> = todo!();

        if let Some(message) = message {
            self.broker_addr.send(message).map_err(RuntimeError::SendError)?;
            channel.send(true).ok();
        } else {
            channel.send(false).ok();
        }
        Ok(())
    }
}
