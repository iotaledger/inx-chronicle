// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{MongoDatabase, MongoDbError},
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
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
pub struct Broker {
    db: MongoDatabase,
}

impl Broker {
    pub fn new(db: MongoDatabase) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Actor for Broker {
    type State = ();
    type Error = BrokerError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<inx::proto::Message> for Broker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        message: inx::proto::Message,
        _data: &mut Self::State,
    ) -> Result<(), Self::Error> {
        debug!("Received Message Event");
        self.db.insert_message_raw(message).await?;
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<inx::proto::Milestone> for Broker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        milestone: inx::proto::Milestone,
        _data: &mut Self::State,
    ) -> Result<(), Self::Error> {
        debug!("Received Milestone Event");
        self.db.insert_milestone(milestone).await?;
        Ok(())
    }
}
