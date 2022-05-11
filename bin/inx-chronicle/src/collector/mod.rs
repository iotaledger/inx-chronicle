// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod solidifier;
pub mod syncer;

use std::collections::{HashMap, VecDeque};

use async_trait::async_trait;
use chronicle::{
    db::{bson::DocError, MongoDb},
    runtime::{
        actor::{addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, Actor},
        config::ConfigureActor,
        error::RuntimeError,
    },
};
use mongodb::bson::document::ValueAccessError;
use solidifier::Solidifier;
use syncer::{Syncer, SyncerConfig};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error(transparent)]
    Doc(#[from] DocError),
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    ValueAccess(#[from] ValueAccessError),
}

#[derive(Debug)]
pub(crate) struct Collector {
    db: MongoDb,
    solidifier_count: usize,
    syncer_config: SyncerConfig,
}

impl Collector {
    const MAX_SOLIDIFIERS: usize = 100;

    pub fn new(db: MongoDb, solidifier_count: usize, syncer_config: SyncerConfig) -> Self {
        Self {
            db,
            solidifier_count: solidifier_count.max(1).min(Self::MAX_SOLIDIFIERS),
            syncer_config,
        }
    }
}

pub(crate) struct CollectorState {
    solidifiers: HashMap<usize, Addr<Solidifier>>,
    syncer_spawned: bool,
}

#[async_trait]
impl Actor for Collector {
    type State = CollectorState;
    type Error = CollectorError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let mut solidifiers = HashMap::new();
        for i in 0..self.solidifier_count {
            solidifiers.insert(
                i,
                cx.spawn_child(Solidifier::new(i, self.db.clone()).with_registration(false))
                    .await,
            );
        }
        Ok(CollectorState {
            solidifiers,
            syncer_spawned: false,
        })
    }
}

#[async_trait]
impl HandleEvent<Report<Solidifier>> for Collector {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Solidifier>,
        state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(report) => match &report.error {
                ActorError::Result(e) => match e {
                    solidifier::SolidifierError::MissingInxRequester => {
                        state
                            .solidifiers
                            .insert(report.actor.id, cx.spawn_child(report.actor).await);
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

#[async_trait]
impl HandleEvent<Report<Syncer>> for Collector {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<Syncer>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match e.error {
                ActorError::Result(_) => {
                    cx.spawn_child(Syncer::new(self.db.clone(), self.syncer_config.clone()))
                        .await;
                }
                ActorError::Panic | ActorError::Aborted => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[cfg(feature = "stardust")]
pub mod stardust {
    use std::collections::HashSet;

    use chronicle::{
        db::model::stardust::{
            message::{MessageMetadata, MessageRecord},
            milestone::MilestoneRecord,
        },
        dto,
    };

    use super::*;

    #[derive(Debug)]
    pub struct MilestoneState {
        pub milestone_index: u32,
        pub process_queue: VecDeque<dto::MessageId>,
        pub visited: HashSet<dto::MessageId>,
    }

    impl MilestoneState {
        pub fn new(milestone_index: u32) -> Self {
            Self {
                milestone_index,
                process_queue: VecDeque::new(),
                visited: HashSet::new(),
            }
        }
    }

    #[derive(Debug)]
    pub struct RequestedMessage<Sender: Actor> {
        raw: Option<inx::proto::RawMessage>,
        metadata: inx::proto::MessageMetadata,
        sender_addr: Addr<Sender>,
        ms_state: MilestoneState,
    }

    impl<Sender: Actor> RequestedMessage<Sender> {
        pub fn new(
            raw: Option<inx::proto::RawMessage>,
            metadata: inx::proto::MessageMetadata,
            sender_addr: Addr<Sender>,
            ms_state: MilestoneState,
        ) -> Self {
            Self {
                raw,
                metadata,
                sender_addr,
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
            log::trace!(
                "Received Stardust Message Event ({})",
                hex::encode(message.message_id.as_ref().unwrap().id.as_slice())
            );
            match MessageRecord::try_from(message) {
                Ok(rec) => {
                    self.db.upsert_message_record(&rec).await?;
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
            log::trace!(
                "Received Stardust Message Referenced Event ({})",
                metadata.milestone_index
            );
            match inx::MessageMetadata::try_from(metadata) {
                Ok(rec) => {
                    let message_id = rec.message_id;
                    self.db
                        .update_message_metadata(&message_id.into(), &MessageMetadata::from(rec))
                        .await?;
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
            cx: &mut ActorContext<Self>,
            milestone: inx::proto::Milestone,
            state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::trace!(
                "Received Stardust Milestone Event ({})",
                milestone.milestone_info.as_ref().unwrap().milestone_index
            );
            match MilestoneRecord::try_from(milestone) {
                Ok(rec) => {
                    self.db.upsert_milestone_record(&rec).await?;
                    // Get or create the milestone state
                    let mut ms_state = MilestoneState::new(rec.milestone_index);
                    ms_state
                        .process_queue
                        .extend(Vec::from(rec.payload.essence.parents).into_iter());
                    if !state.syncer_spawned {
                        cx.spawn_child(Syncer::new(self.db.clone(), self.syncer_config.clone()))
                            .await;
                        state.syncer_spawned = true;
                    }
                    state
                        .solidifiers
                        // Divide solidifiers fairly by milestone
                        .get(&(rec.milestone_index as usize % self.solidifier_count))
                        // Unwrap: We never remove solidifiers, so they should always exist
                        .unwrap()
                        .send(ms_state)?;
                }
                Err(e) => {
                    log::error!("Could not read milestone: {:?}", e);
                }
            }
            Ok(())
        }
    }

    #[async_trait]
    impl<Sender: Actor> HandleEvent<RequestedMessage<Sender>> for Collector
    where
        Sender: 'static + HandleEvent<MilestoneState>,
    {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            RequestedMessage {
                raw,
                metadata,
                sender_addr,
                ms_state,
            }: RequestedMessage<Sender>,
            _solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match raw {
                Some(raw) => {
                    log::trace!("Received Stardust Requested Message and Metadata");
                    match MessageRecord::try_from((raw, metadata)) {
                        Ok(rec) => {
                            self.db.upsert_message_record(&rec).await?;
                            // Send this directly to the solidifier that requested it
                            sender_addr.send(ms_state)?;
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
                            self.db
                                .update_message_metadata(&message_id.into(), &MessageMetadata::from(rec))
                                .await?;
                            // Send this directly to the solidifier that requested it
                            sender_addr.send(ms_state)?;
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
