// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{MongoDatabase, MongoDbError},
    runtime::{
        actor::{context::ActorContext, envelope::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use log::debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
    #[error(transparent)]
    MongoDbError(#[from] MongoDbError),
}

#[derive(Debug)]
pub struct Broker;

#[async_trait]
impl Actor for Broker {
    type Data = MongoDatabase;
    type Error = BrokerError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::Data, Self::Error>
    where
        Self: 'static + Sized + Send + Sync,
    {
        Ok(cx.link_resource().await?)
    }
}

#[async_trait]
impl HandleEvent<inx::proto::Message> for Broker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        message: inx::proto::Message,
        db: &mut Self::Data,
    ) -> Result<(), Self::Error> {
        debug!("Received Message Event");
        db.insert_message_raw(message).await?;
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<inx::proto::Milestone> for Broker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        milestone: inx::proto::Milestone,
        db: &mut Self::Data,
    ) -> Result<(), Self::Error> {
        debug!("Received Milestone Event");
        db.insert_milestone(milestone).await?;
        Ok(())
    }
}
