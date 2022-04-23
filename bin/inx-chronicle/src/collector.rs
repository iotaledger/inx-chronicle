// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, VecDeque};

use async_trait::async_trait;
use chronicle::{
    bson::DocError,
    db::MongoDatabase,
    runtime::{
        actor::{addr::Addr, context::ActorContext, error::ActorError, event::HandleEvent, report::Report, Actor},
        error::RuntimeError,
    },
};
use mongodb::bson::document::ValueAccessError;
use thiserror::Error;

use crate::{archiver::Archiver, solidifier::Solidifier};

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("The archiver is missing")]
    ArchiverMissing,
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
pub struct MilestoneState {
    pub milestone_index: u32,
    pub parents: HashMap<String, Option<Vec<String>>>,
    pub process_queue: VecDeque<String>,
}

impl MilestoneState {
    pub fn new(milestone_index: u32) -> Self {
        Self {
            milestone_index,
            parents: HashMap::new(),
            process_queue: VecDeque::new(),
        }
    }
}

pub struct Collector {
    db: MongoDatabase,
    archiver_addr: Addr<Archiver>,
    solidifier_count: usize,
    milestones: HashMap<u32, MilestoneState>,
    parents: HashMap<String, Vec<String>>,
}

impl Collector {
    pub fn new(db: MongoDatabase, archiver_addr: Addr<Archiver>, solidifier_count: usize) -> Self {
        Self {
            db,
            archiver_addr,
            solidifier_count,
            milestones: HashMap::new(),
            parents: HashMap::new(),
        }
    }
}

#[async_trait]
impl Actor for Collector {
    type State = HashMap<usize, Addr<Solidifier>>;
    type Error = CollectorError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let mut solidifiers = HashMap::new();
        for i in 0..self.solidifier_count.max(1) {
            solidifiers.insert(
                i,
                cx.spawn_actor_supervised(Solidifier::new(i, self.db.clone(), self.archiver_addr.clone()))
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
            Ok(_) => {
                cx.shutdown();
            }
            Err(e) => match e.error {
                ActorError::Result(_) => {
                    if self.archiver_addr.is_closed() {
                        return Err(CollectorError::ArchiverMissing);
                    } else {
                        solidifiers.insert(e.actor.id, cx.spawn_actor_supervised(e.actor).await);
                    }
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[cfg(feature = "stardust")]
mod stardust {
    use chronicle::stardust::{milestone::MilestoneIndex, MessageId};

    use super::*;

    #[derive(Debug)]
    pub enum CollectorEvent {
        /// A message with its parents
        Message {
            message_id: MessageId,
            parents: Vec<MessageId>,
        },
        /// An indicator that a message was referenced by a milestone
        MessageReferenced {
            milestone_index: MilestoneIndex,
            message_id: MessageId,
        },
        /// A milestone
        Milestone {
            milestone_index: MilestoneIndex,
            message_id: MessageId,
        },
    }

    #[cfg(feature = "stardust")]
    #[async_trait]
    impl HandleEvent<CollectorEvent> for Collector {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            event: CollectorEvent,
            solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match event {
                CollectorEvent::Message { message_id, parents } => {
                    self.parents.insert(
                        message_id.to_string(),
                        parents.iter().map(|id| id.to_string()).collect(),
                    );
                }
                CollectorEvent::MessageReferenced {
                    milestone_index,
                    message_id,
                } => match self.parents.remove(&message_id.to_string()) {
                    Some(parents) => {
                        self.milestones
                            .entry(milestone_index.0)
                            .or_insert_with(|| MilestoneState::new(milestone_index.0))
                            .parents
                            .insert(message_id.to_string(), Some(parents));
                    }
                    None => {
                        // We got a message referenced but no message
                        // so we can just let the solidifier eventually request it
                        self.milestones
                            .entry(milestone_index.0)
                            .or_insert_with(|| MilestoneState::new(milestone_index.0))
                            .parents
                            .insert(message_id.to_string(), None);
                    }
                },
                CollectorEvent::Milestone {
                    milestone_index,
                    message_id,
                } => {
                    // Get or create the milestone state
                    let mut state = self
                        .milestones
                        .remove(&milestone_index.0)
                        .unwrap_or_else(|| MilestoneState::new(milestone_index.0));
                    state.process_queue.push_back(message_id.to_string());
                    solidifiers
                        // Divide solidifiers fairly by milestone
                        .get(&(milestone_index.0 as usize % self.solidifier_count))
                        // Unwrap: We never remove solidifiers, so they should always exist
                        .unwrap()
                        .send(state)
                        .map_err(RuntimeError::SendError)?;
                }
            }
            Ok(())
        }
    }
}
