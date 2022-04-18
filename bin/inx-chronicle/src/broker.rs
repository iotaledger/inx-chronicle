// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
#[cfg(feature = "chrysalis")]
use chronicle::db::model::chrysalis;
#[cfg(feature = "stardust")]
use chronicle::db::model::stardust;
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

#[cfg(feature = "stardust")]
#[async_trait]
impl HandleEvent<inx::proto::Message> for Broker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        message: inx::proto::Message,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        debug!("Received Stardust Message Event");
        match stardust::message::MessageRecord::try_from(message) {
            Ok(rec) => self.db.upsert_one(rec).await?,
            Err(e) => {
                log::error!("Could not read message: {:?}", e);
            }
        };
        Ok(())
    }
}

#[cfg(feature = "stardust")]
#[async_trait]
impl HandleEvent<inx::proto::Milestone> for Broker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        milestone: inx::proto::Milestone,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        debug!("Received Stardust Milestone Event");
        match stardust::milestone::MilestoneRecord::try_from(milestone) {
            Ok(rec) => self.db.upsert_one(rec).await?,
            Err(e) => {
                log::error!("Could not read milestone: {:?}", e);
            }
        };
        Ok(())
    }
}

#[cfg(feature = "chrysalis")]
#[async_trait]
impl HandleEvent<chrysalis::message::MessageRecord> for Broker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        message: chrysalis::message::MessageRecord,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        debug!("Received Chrysalis Message Event");
        self.db.upsert_one(message).await?;
        Ok(())
    }
}

#[cfg(feature = "chrysalis")]
#[async_trait]
impl HandleEvent<chrysalis::milestone::MilestoneRecord> for Broker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        milestone: chrysalis::milestone::MilestoneRecord,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        debug!("Received Chrysalis Milestone Event");
        self.db.upsert_one(milestone).await?;
        Ok(())
    }
}
