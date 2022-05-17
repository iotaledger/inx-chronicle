// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

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
use serde::{Deserialize, Serialize};
use solidifier::Solidifier;
use thiserror::Error;

pub mod solidifier;

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

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub solidifier_count: usize,
}

#[allow(dead_code)]
impl CollectorConfig {
    const MAX_SOLIDIFIERS: usize = 100;

    pub fn set_solidifier_count(&mut self, solidifier_count: usize) {
        self.solidifier_count = solidifier_count.clamp(1, Self::MAX_SOLIDIFIERS);
    }
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self { solidifier_count: 10 }
    }
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

#[async_trait]
impl Actor for Collector {
    type State = HashMap<usize, Addr<Solidifier>>;
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
                    solidifier::SolidifierError::MissingInxRequester => {
                        solidifiers.insert(report.actor.id, cx.spawn_child(report.actor).await);
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
    use std::{collections::HashSet, time::Instant};

    use chronicle::{
        db::model::stardust::{
            message::{MessageMetadata, MessageRecord},
            milestone::MilestoneRecord,
        },
        dto,
    };

    use super::*;
    use crate::inx::syncer::InxSyncer;

    #[derive(Debug)]
    pub struct MilestoneState {
        pub milestone_index: u32,
        pub process_queue: VecDeque<dto::MessageId>,
        pub visited: HashSet<dto::MessageId>,
        pub time: Instant,
        pub syncer_addr: Option<Addr<InxSyncer>>,
    }

    impl MilestoneState {
        pub fn new(milestone_index: u32, parents: Vec<dto::MessageId>, syncer_addr: Option<Addr<InxSyncer>>) -> Self {
            Self {
                milestone_index,
                process_queue: parents.into(),
                visited: HashSet::new(),
                time: Instant::now(),
                syncer_addr,
            }
        }
    }

    // TODO: investigate why `visited.len() == 0` for synced milestones
    impl Drop for MilestoneState {
        fn drop(&mut self) {
            log::trace!(
                "Solidification state of milestone '{}' dropped after {}s with {} visited and {} remaining messages.",
                self.milestone_index,
                self.time.elapsed().as_secs_f32(),
                self.visited.len(),
                self.process_queue.len()
            );
        }
    }

    #[derive(Debug)]
    pub struct RequestedMessage {
        raw: Option<inx::proto::RawMessage>,
        metadata: inx::proto::MessageMetadata,
        solidifier_addr: Addr<Solidifier>,
        ms_state: MilestoneState,
    }

    impl RequestedMessage {
        pub fn new(
            raw: Option<inx::proto::RawMessage>,
            metadata: inx::proto::MessageMetadata,
            solidifier_addr: Addr<Solidifier>,
            ms_state: MilestoneState,
        ) -> Self {
            Self {
                raw,
                metadata,
                solidifier_addr,
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
            match MessageRecord::try_from(message) {
                Ok(rec) => {
                    log::trace!("Received Stardust Message Event ({})", rec.message.id.to_hex());
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
            match inx::MessageMetadata::try_from(metadata) {
                Ok(rec) => {
                    log::trace!("Received Stardust Message Referenced Event ({})", rec.milestone_index);
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
            _cx: &mut ActorContext<Self>,
            milestone: inx::proto::Milestone,
            solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match MilestoneRecord::try_from(milestone) {
                Ok(rec) => {
                    log::trace!("Received Stardust Milestone Event ({})", rec.milestone_index);
                    self.db.upsert_milestone_record(&rec).await?;

                    // Get or create the milestone state
                    let parents = Vec::from(rec.payload.essence.parents);
                    let state = MilestoneState::new(rec.milestone_index, parents, None);

                    solidifiers
                        // Divide solidifiers fairly by milestone
                        .get(&(rec.milestone_index as usize % self.config.solidifier_count))
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
                solidifier_addr,
                ms_state,
            }: RequestedMessage,
            _solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match raw {
                Some(raw) => {
                    log::trace!("Received Stardust Requested Message and Metadata");
                    match MessageRecord::try_from((raw, metadata)) {
                        Ok(rec) => {
                            self.db.upsert_message_record(&rec).await?;
                            // Send this directly to the solidifier that requested it
                            solidifier_addr.send(ms_state)?;
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
                            solidifier_addr.send(ms_state)?;
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

    pub struct RequestedMilestone {
        milestone: inx::proto::Milestone,
        syncer_addr: Addr<InxSyncer>,
    }

    impl RequestedMilestone {
        pub fn new(milestone: inx::proto::Milestone, syncer_addr: Addr<InxSyncer>) -> Self {
            Self { milestone, syncer_addr }
        }
    }

    #[async_trait]
    impl HandleEvent<RequestedMilestone> for Collector {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            RequestedMilestone { milestone, syncer_addr }: RequestedMilestone,
            solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match MilestoneRecord::try_from(milestone) {
                Ok(rec) => {
                    // TODO: back to trace
                    log::warn!("Received Stardust Requested Milestone ({})", rec.milestone_index);
                    self.db.upsert_milestone_record(&rec).await?;

                    // Get or create the milestone state
                    let parents = Vec::from(rec.payload.essence.parents);
                    let state = MilestoneState::new(rec.milestone_index, parents, Some(syncer_addr));

                    solidifiers
                        // Divide solidifiers fairly by milestone
                        .get(&(rec.milestone_index as usize % self.config.solidifier_count))
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
}
