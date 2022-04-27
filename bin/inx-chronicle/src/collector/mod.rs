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
use lru::LruCache;
use mongodb::bson::document::ValueAccessError;
use solidifier::Solidifier;
use thiserror::Error;

use crate::{archiver::Archiver, inx::InxRequester, ADDRESS_REGISTRY};

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
    #[cfg(feature = "stardust")]
    stardust_messages: LruCache<chronicle::stardust::MessageId, chronicle::stardust::Message>,
    #[cfg(feature = "stardust")]
    stardust_milestones: HashMap<u32, stardust::MilestoneState>,
}

impl Collector {
    pub fn new(db: MongoDatabase, solidifier_count: usize) -> Self {
        Self {
            db,
            solidifier_count,
            #[cfg(feature = "stardust")]
            stardust_milestones: HashMap::new(),
            #[cfg(feature = "stardust")]
            stardust_messages: LruCache::new(1000),
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
                        if ADDRESS_REGISTRY.get::<Archiver>().await.is_none() {
                            cx.delay(<Report<Solidifier>>::Error(report), None)?;
                        } else {
                            solidifiers.insert(report.actor.id, cx.spawn_actor_supervised(report.actor).await);
                        }
                    }
                    solidifier::SolidifierError::MissingInxRequester => {
                        if ADDRESS_REGISTRY.get::<InxRequester>().await.is_none() {
                            cx.delay(<Report<Solidifier>>::Error(report), None)?;
                        } else {
                            solidifiers.insert(report.actor.id, cx.spawn_actor_supervised(report.actor).await);
                        }
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

    use chronicle::stardust::{milestone::MilestoneIndex, Message, MessageId};

    use super::*;

    #[derive(Debug)]
    pub struct MilestoneState {
        pub milestone_index: u32,
        pub process_queue: VecDeque<MessageId>,
        pub messages: HashMap<MessageId, Message>,
        pub raw_messages: BTreeMap<MessageId, Vec<u8>>,
    }

    impl MilestoneState {
        pub fn new(milestone_index: u32) -> Self {
            Self {
                milestone_index,
                process_queue: VecDeque::new(),
                messages: HashMap::new(),
                raw_messages: BTreeMap::new(),
            }
        }
    }

    #[derive(Debug)]
    pub enum CollectorEvent {
        Message(Message),
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

    #[async_trait]
    impl HandleEvent<CollectorEvent> for Collector {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            event: CollectorEvent,
            solidifiers: &mut Self::State,
        ) -> Result<(), Self::Error> {
            match event {
                CollectorEvent::Message(message) => {
                    self.stardust_messages.put(message.id(), message);
                }
                CollectorEvent::MessageReferenced {
                    milestone_index,
                    message_id,
                } => {
                    let ms_state = self
                        .stardust_milestones
                        .entry(milestone_index.0)
                        .or_insert_with(|| MilestoneState::new(milestone_index.0));

                    if let Some(message) = self.stardust_messages.pop(&message_id) {
                        ms_state.messages.insert(message_id, message);
                    }
                }
                CollectorEvent::Milestone {
                    milestone_index,
                    message_id,
                } => {
                    // Get or create the milestone state
                    let mut state = self
                        .stardust_milestones
                        .remove(&milestone_index.0)
                        .unwrap_or_else(|| MilestoneState::new(milestone_index.0));
                    state.process_queue.push_back(message_id);
                    solidifiers
                        // Divide solidifiers fairly by milestone
                        .get(&(milestone_index.0 as usize % self.solidifier_count))
                        // Unwrap: We never remove solidifiers, so they should always exist
                        .unwrap()
                        .send(state)?;
                }
            }
            Ok(())
        }
    }
}
