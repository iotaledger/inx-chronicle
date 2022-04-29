// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
#[cfg(feature = "stardust")]
use chronicle::db::model::stardust;
use chronicle::{
    db::MongoDb,
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error(transparent)]
    MongoDbError(#[from] mongodb::error::Error),
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
}

#[derive(Debug)]
pub struct Broker {
    db: MongoDb,
}

impl Broker {
    pub fn new(db: MongoDb) -> Self {
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
        log::trace!("Received Stardust Message Event");
        match stardust::message::MessageRecord::try_from(message) {
            Ok(rec) => {
                self.db.upsert_message_record(&rec).await?;
            }
            Err(e) => {
                log::warn!("Could not read message: {:?}", e);
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
        log::debug!("Received Stardust Milestone Event");
        match stardust::milestone::MilestoneRecord::try_from(milestone) {
            Ok(rec) => {
                self.db.upsert_milestone_record(&rec).await?;
            }
            Err(e) => {
                log::warn!("Could not read milestone: {:?}", e);
            }
        };
        Ok(())
    }
}
