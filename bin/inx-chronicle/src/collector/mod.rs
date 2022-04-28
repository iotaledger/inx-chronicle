// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, VecDeque};

use async_trait::async_trait;
use chronicle::{
    bson::DocError,
    db::{MongoDatabase, MongoDbError},
    runtime::{
        actor::{addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, Actor},
        error::RuntimeError,
    },
};
use mongodb::bson::document::ValueAccessError;
use solidifier::Solidifier;
use thiserror::Error;

pub mod solidifier;

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error(transparent)]
    Doc(#[from] DocError),
    #[error(transparent)]
    MongoDb(#[from] MongoDbError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    ValueAccess(#[from] ValueAccessError),
}

#[derive(Debug)]
pub struct Collector {
    db: MongoDatabase,
    solidifier_count: usize,
}

impl Collector {
    pub fn new(db: MongoDatabase, solidifier_count: usize) -> Self {
        Self { db, solidifier_count }
    }
}

#[async_trait]
impl Actor for Collector {
    type State = HashMap<usize, Addr<Solidifier>>;
    type Error = CollectorError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let mut solidifiers = HashMap::new();
        for i in 0..self.solidifier_count.max(1) {
            solidifiers.insert(i, cx.spawn_actor_supervised(Solidifier::new(i, self.db.clone())).await);
        }
        Ok(solidifiers)
    }
}

#[async_trait]
impl HandleEvent<Report<Solidifier>> for Collector {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Solidifier>,
        solidifiers: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    solidifier::SolidifierError::MissingArchiver => {
                        solidifiers.insert(report.actor.id, cx.spawn_actor_supervised(report.actor).await);
                    }
                    solidifier::SolidifierError::MissingInxRequester => {
                        solidifiers.insert(report.actor.id, cx.spawn_actor_supervised(report.actor).await);
                    }
                    // TODO: Maybe map Solidifier errors to Collector errors and return them?
                    _ => {
                        cx.shutdown();
                    }
                },
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[cfg(feature = "stardust")]
pub mod stardust {
    use std::collections::BTreeMap;

    use chronicle::{
        db::model::stardust::{
            message::{MessageMetadata, MessageRecord},
            milestone::MilestoneRecord,
        },
        stardust::MessageId,
    };
    use mongodb::bson::{doc, to_document};

    use super::*;

    #[derive(Debug)]
    pub struct MilestoneState {
        pub milestone_index: u32,
        pub process_queue: VecDeque<MessageId>,
        pub messages: BTreeMap<MessageId, Vec<u8>>,
    }

    impl MilestoneState {
        pub fn new(milestone_index: u32) -> Self {
            Self {
                milestone_index,
                process_queue: VecDeque::new(),
                messages: BTreeMap::new(),
            }
        }
    }

    #[derive(Debug)]
    pub struct RequestedMessage {
        raw: Option<inx::proto::RawMessage>,
        metadata: inx::proto::MessageMetadata,
        solidifier: Addr<Solidifier>,
        ms_state: MilestoneState,
    }

    impl RequestedMessage {
        pub fn new(
            raw: Option<inx::proto::RawMessage>,
            metadata: inx::proto::MessageMetadata,
            solidifier: Addr<Solidifier>,
            ms_state: MilestoneState,
        ) -> Self {
            Self {
                raw,
                metadata,
                solidifier,
                ms_state,
            }
        }
    }

    #[async_trait]
    impl HandleEvent<inx::proto::Message> for Collector {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            message: inx::proto::Message,
            _solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::trace!("Received Stardust Message Event");
            match MessageRecord::try_from(message) {
                Ok(rec) => {
                    self.db.upsert_one(&rec).await?;
                }
                Err(e) => {
                    log::error!("Could not read message: {:?}", e);
                }
            };
            Ok(())
        }
    }

    #[async_trait]
    impl HandleEvent<inx::proto::MessageMetadata> for Collector {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            metadata: inx::proto::MessageMetadata,
            _solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::trace!("Received Stardust Message Referenced Event");
            match inx::MessageMetadata::try_from(metadata) {
                Ok(rec) => {
                    let message_id = rec.message_id;
                    match to_document(&MessageMetadata::from(rec)) {
                        Ok(doc) => {
                            self.db
                                .collection::<MessageRecord>()
                                .update_one(
                                    doc! { "message_id": message_id.to_string() },
                                    doc! { "$set": { "metadata": doc } },
                                    None,
                                )
                                .await
                                .map_err(MongoDbError::DatabaseError)?;
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
    impl HandleEvent<inx::proto::Milestone> for Collector {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            milestone: inx::proto::Milestone,
            solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::trace!("Received Stardust Milestone Event");
            match MilestoneRecord::try_from(milestone) {
                Ok(rec) => {
                    self.db.upsert_one(&rec).await?;
                    // Get or create the milestone state
                    let mut state = MilestoneState::new(rec.milestone_index);
                    state.process_queue.extend(rec.payload.essence().parents().iter());
                    solidifiers
                        // Divide solidifiers fairly by milestone
                        .get(&(rec.milestone_index as usize % self.solidifier_count))
                        // Unwrap: We never remove solidifiers, so they should always exist
                        .unwrap()
                        .send(state)?;
                }
                Err(e) => {
                    log::error!("Could not read milestone: {:?}", e);
                }
            }
            Ok(())
        }
    }

    #[async_trait]
    impl HandleEvent<RequestedMessage> for Collector {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            RequestedMessage {
                raw,
                metadata,
                solidifier,
                ms_state,
            }: RequestedMessage,
            _solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match raw {
                Some(raw) => {
                    log::trace!("Received Stardust Requested Message and Metadata");
                    match MessageRecord::try_from((raw, metadata)) {
                        Ok(rec) => {
                            self.db.upsert_one(&rec).await?;
                            // Send this directly to the solidifier that requested it
                            solidifier.send(ms_state)?;
                        }
                        Err(e) => {
                            log::error!("Could not read message: {:?}", e);
                        }
                    };
                }
                None => {
                    log::trace!("Received Stardust Requested Metadata");
                    match inx::MessageMetadata::try_from(metadata) {
                        Ok(rec) => {
                            let message_id = rec.message_id;
                            match to_document(&MessageMetadata::from(rec)) {
                                Ok(doc) => {
                                    self.db
                                        .collection::<MessageRecord>()
                                        .update_one(
                                            doc! { "message_id": message_id.to_string() },
                                            doc! { "$set": { "metadata": doc } },
                                            None,
                                        )
                                        .await
                                        .map_err(MongoDbError::DatabaseError)?;
                                    // Send this directly to the solidifier that requested it
                                    solidifier.send(ms_state)?;
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
                }
            }

            Ok(())
        }
    }
}
