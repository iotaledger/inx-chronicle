// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{model::stardust, MongoDatabase, MongoDbError},
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
        log::trace!("Received Message Event");
        // TODO: How do we handle chrysalis vs stardust messages?
        match stardust::message::MessageRecord::try_from(message) {
            Ok(rec) => self.db.upsert_one(rec).await?,
            Err(e) => {
                log::error!("Could not read message: {:?}", e);
            }
        };
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
        // TODO: How do we handle chrysalis vs stardust milestones?
        match stardust::milestone::MilestoneRecord::try_from(milestone) {
            Ok(rec) => self.db.upsert_one(rec).await?,
            Err(e) => {
                log::error!("Could not read milestone: {:?}", e);
            }
        };
        Ok(())
    }
}
