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
use thiserror::Error;

use crate::collector::Collector;

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error(transparent)]
    MongoDbError(#[from] MongoDbError),
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
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
mod stardust {
    use bee_message_stardust::milestone::MilestoneIndex;
    use chronicle::{
        db::model::stardust::{
            self,
            message::{MessageMetadata, MessageRecord},
        },
        runtime::actor::addr::Addr,
    };
    use mongodb::bson::{doc, to_document};

    use super::*;
    use crate::{
        collector::{
            solidifier::Solidifier,
            stardust::{CollectorEvent, MilestoneState},
        },
        ADDRESS_REGISTRY,
    };

    #[async_trait]
    impl HandleEvent<inx::proto::Message> for Broker {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            message: inx::proto::Message,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::debug!("Received Stardust Message Event");
            match MessageRecord::try_from(message) {
                Ok(rec) => {
                    self.db.upsert_one(&rec).await?;
                    ADDRESS_REGISTRY
                        .get::<Collector>()
                        .await
                        .send(CollectorEvent::Message(rec.message))
                        .map_err(RuntimeError::SendError)?;
                }
                Err(e) => {
                    log::error!("Could not read message: {:?}", e);
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
            message: inx::proto::MessageMetadata,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::debug!("Received Stardust Message Referenced Event");
            match inx::MessageMetadata::try_from(message) {
                Ok(rec) => {
                    let message_id = rec.message_id;
                    let milestone_index = MilestoneIndex::from(rec.milestone_index);
                    match to_document(&MessageMetadata::from(rec)) {
                        Ok(doc) => {
                            self.db
                                .collection::<MessageRecord>()
                                .update_one(doc! {"message_id": message_id.to_string()}, doc! { "$set": doc }, None)
                                .await
                                .map_err(MongoDbError::DatabaseError)?;
                            ADDRESS_REGISTRY
                                .get::<Collector>()
                                .await
                                .send(CollectorEvent::MessageReferenced {
                                    milestone_index,
                                    message_id,
                                })
                                .map_err(RuntimeError::SendError)?;
                        }
                        Err(e) => {
                            log::error!("Could not read message metadata: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Could not read message metadata: {:?}", e);
                }
            };
            Ok(())
        }
    }

    #[async_trait]
    impl
        HandleEvent<(
            inx::proto::RawMessage,
            inx::proto::MessageMetadata,
            Addr<Solidifier>,
            MilestoneState,
        )> for Broker
    {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            (message, metadata, solidifier, ms_state): (
                inx::proto::RawMessage,
                inx::proto::MessageMetadata,
                Addr<Solidifier>,
                MilestoneState,
            ),
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::debug!("Received Stardust Requested Message and Metadata");
            match MessageRecord::try_from((message, metadata)) {
                Ok(rec) => {
                    self.db.upsert_one(&rec).await?;
                    // Send this directly to the solidifier that requested it
                    solidifier.send(ms_state).map_err(RuntimeError::SendError)?;
                }
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
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::debug!("Received Stardust Milestone Event");
            match stardust::milestone::MilestoneRecord::try_from(milestone) {
                Ok(rec) => {
                    self.db.upsert_one(&rec).await?;
                    ADDRESS_REGISTRY
                        .get::<Collector>()
                        .await
                        .send(CollectorEvent::Milestone {
                            milestone_index: MilestoneIndex::from(rec.milestone_index),
                            message_id: rec.message_id,
                        })
                        .map_err(RuntimeError::SendError)?;
                }
                Err(e) => {
                    log::error!("Could not read milestone: {:?}", e);
                }
            };
            Ok(())
        }
    }
}
