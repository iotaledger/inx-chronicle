// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{self, MongoDatabase, MongoDbError},
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
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
        match message.try_into() {
            Ok((inx::Message { message_id, message }, raw)) => {
                self.db
                    .insert_one(db::model::stardust::Message {
                        message_id,
                        message,
                        raw,
                    })
                    .await?
            }
            Err(e) => {
                log::warn!("Could not read message: {:?}", e);
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
        log::trace!("Received Milestone Event");
        match milestone.try_into() {
            Ok(inx::Milestone {
                message_id,
                milestone_id,
                milestone_index,
                milestone_timestamp,
            }) => {
                self.db
                    .insert_one(db::model::stardust::Milestone {
                        message_id,
                        milestone_id,
                        milestone_index,
                        milestone_timestamp,
                    })
                    .await?
            }
            Err(e) => {
                log::warn!("Could not read milestone: {:?}", e);
            }
        };
        Ok(())
    }
}
