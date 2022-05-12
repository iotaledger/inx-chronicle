// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
pub mod solidifier;

use std::collections::{HashMap, VecDeque};

use async_trait::async_trait;
use chronicle::{
    db::{bson::DocError, MongoDb},
    runtime::{Actor, ActorContext, ActorError, Addr, ConfigureActor, HandleEvent, Report, RuntimeError},
    types::{ledger::Metadata, stardust::message::MessageId},
};
pub use config::CollectorConfig;
use mongodb::bson::document::ValueAccessError;
use solidifier::Solidifier;
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
pub struct Collector {
    db: MongoDb,
    config: CollectorConfig,
}

impl Collector {
    pub fn new(db: MongoDb, config: CollectorConfig) -> Self {
        Self { db, config }
    }
}

pub struct CollectorState {
    solidifiers: HashMap<usize, Addr<Solidifier>>,
}

#[async_trait]
impl Actor for Collector {
    type State = CollectorState;
    type Error = CollectorError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let mut solidifiers = HashMap::new();
        for i in 0..self.config.solidifier_count {
            solidifiers.insert(
                i,
                cx.spawn_child(Solidifier::new(i, self.db.clone()).with_registration(false))
                    .await,
            );
        }
        Ok(CollectorState { solidifiers })
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
                    #[cfg(all(feature = "stardust", feature = "inx"))]
                    solidifier::SolidifierError::MissingStardustInxRequester => {
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

#[cfg(all(feature = "stardust", feature = "inx"))]
pub mod stardust_inx {
    use std::collections::HashSet;

    use chronicle::db::model::stardust::{message::MessageRecord, milestone::MilestoneRecord};

    use super::*;

    #[derive(Debug)]
    pub struct MilestoneState {
        pub milestone_index: u32,
        pub process_queue: VecDeque<MessageId>,
        pub visited: HashSet<MessageId>,
        pub sender: Option<tokio::sync::oneshot::Sender<u32>>,
    }

    impl MilestoneState {
        pub fn new(milestone_index: u32) -> Self {
            Self {
                milestone_index,
                process_queue: VecDeque::new(),
                visited: HashSet::new(),
                sender: None,
            }
        }

        pub fn requested(milestone_index: u32, sender: tokio::sync::oneshot::Sender<u32>) -> Self {
            Self {
                milestone_index,
                process_queue: VecDeque::new(),
                visited: HashSet::new(),
                sender: Some(sender),
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

    pub struct RequestedMilestone(inx::proto::Milestone, tokio::sync::oneshot::Sender<u32>);
    impl RequestedMilestone {
        pub fn new(milestone: inx::proto::Milestone, sender: tokio::sync::oneshot::Sender<u32>) -> Self {
            Self(milestone, sender)
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
                prefix_hex::encode(message.message_id.as_ref().unwrap().id.as_slice())
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
                        .update_message_metadata(&message_id.into(), &Metadata::from(rec))
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
            _cx: &mut ActorContext<Self>,
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
                    state
                        .solidifiers
                        // Divide solidifiers fairly by milestone
                        .get(&(rec.milestone_index as usize % self.config.solidifier_count))
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
                                .update_message_metadata(&message_id.into(), &Metadata::from(rec))
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

    #[async_trait]
    impl HandleEvent<RequestedMilestone> for Collector {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            RequestedMilestone(milestone, sender): RequestedMilestone,
            state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::trace!(
                "Received Stardust Requested Milestone Event ({})",
                milestone.milestone_info.as_ref().unwrap().milestone_index
            );
            match MilestoneRecord::try_from(milestone) {
                Ok(rec) => {
                    self.db.upsert_milestone_record(&rec).await?;
                    // Get or create the milestone state
                    let mut ms_state = MilestoneState::requested(rec.milestone_index, sender);
                    ms_state
                        .process_queue
                        .extend(Vec::from(rec.payload.essence.parents).into_iter());
                    state
                        .solidifiers
                        // Divide solidifiers fairly by milestone
                        .get(&(rec.milestone_index as usize % self.config.solidifier_count))
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
}
