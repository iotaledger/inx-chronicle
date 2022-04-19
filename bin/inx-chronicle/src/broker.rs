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
use mongodb::{bson::{self},};

const MILLISECONDS_PER_SECOND: i64 = 1000;

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
        log::trace!("Received `Message` event");
        match message.try_into() {
            Ok((inx::Message { message_id, message }, raw)) => {
                self.db
                    .insert_one(db::model::stardust::Message {
                        message_id,
                        message,
                        metadata: None,
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
impl HandleEvent<inx::proto::MessageMetadata> for Broker {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        metadata: inx::proto::MessageMetadata,
        _data: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received `Metadata` event");
        match TryInto::<inx::MessageMetadata>::try_into(metadata) {
            Ok(metadata) => {
                let metadata_model = db::model::stardust::Metadata {
                    is_solid: metadata.is_solid,
                    should_promote: metadata.should_promote,
                    should_reattach: metadata.should_reattach,
                    referenced_by_milestone_index: metadata.referenced_by_milestone_index,
                    ledger_inclusion_state: metadata.ledger_inclusion_state.into(),
                    conflict_reason: metadata.conflict_reason.into(),
                    milestone_index: metadata.milestone_index,
                };
                self.db.update_metadata::<db::model::stardust::Message>(metadata.message_id, metadata_model).await?;
            }
            Err(e) => {
                log::warn!("Could not read metadata: {:?}", e);
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
        log::trace!("Received `Milestone` event");
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
                        milestone_timestamp: bson::DateTime::from_millis(milestone_timestamp as i64 * MILLISECONDS_PER_SECOND )
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
