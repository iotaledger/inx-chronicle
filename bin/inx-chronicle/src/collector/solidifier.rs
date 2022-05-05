// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use mongodb::bson::document::ValueAccessError;
use thiserror::Error;

use crate::syncer::Syncer;

#[derive(Debug, Error)]
pub enum SolidifierError {
    #[error("the syncer is missing")]
    MissingSyncer,
    #[cfg(feature = "stardust")]
    #[error("the INX requester is missing")]
    MissingInxRequester,
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    ValueAccess(#[from] ValueAccessError),
}

#[derive(Debug)]
pub struct Solidifier {
    pub id: usize,
    db: MongoDb,
}

impl Solidifier {
    pub fn new(id: usize, db: MongoDb) -> Self {
        Self { id, db }
    }
}

#[async_trait]
impl Actor for Solidifier {
    type State = ();
    type Error = SolidifierError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[cfg(feature = "stardust")]
mod stardust {
    use chronicle::db::model::sync::SyncRecord;

    use super::*;
    use crate::{
        collector::stardust::MilestoneState,
        inx::{InxRequest, InxWorker},
    };

    #[async_trait]
    impl HandleEvent<MilestoneState> for Solidifier {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            mut ms_state: MilestoneState,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            // Process by iterating the queue until we either complete the milestone or fail to find a message
            while let Some(message_id) = ms_state.process_queue.front() {
                // First check if we already processed this message in this run
                if ms_state.visited.contains(message_id) {
                    ms_state.process_queue.pop_front();
                } else {
                    // Try the database
                    match self.db.get_message(message_id).await? {
                        Some(message_rec) => {
                            match message_rec
                                .metadata
                                .map(|metadata| metadata.referenced_by_milestone_index)
                            {
                                Some(ms_index) => {
                                    log::trace!(
                                        "Message {} is referenced by milestone {}",
                                        message_id.to_hex(),
                                        ms_index
                                    );

                                    // We add the current message to the list of visited messages.
                                    ms_state.visited.insert(message_id.clone());

                                    // We may have reached a different milestone, in which case there is nothing to
                                    // do for this message
                                    if ms_state.milestone_index == ms_index {
                                        let parents = Vec::from(message_rec.message.parents);
                                        ms_state.process_queue.extend(parents);
                                    }
                                    ms_state.process_queue.pop_front();
                                }
                                // If the message has not been referenced, we can't proceed
                                None => {
                                    log::trace!("Requesting metadata for message {}", message_id.to_hex());
                                    // Send the state and everything. If the requester finds the message, it will circle
                                    // back.
                                    cx.addr::<InxWorker>()
                                        .await
                                        .send(InxRequest::get_metadata(
                                            message_id.clone(),
                                            cx.handle().clone(),
                                            ms_state,
                                        ))
                                        .map_err(|_| SolidifierError::MissingInxRequester)?;
                                    return Ok(());
                                }
                            }
                        }
                        // Otherwise, send a message to the requester
                        None => {
                            log::trace!("Requesting message {}", message_id.to_hex());
                            // Send the state and everything. If the requester finds the message, it will circle
                            // back.
                            cx.addr::<InxWorker>()
                                .await
                                .send(InxRequest::get_message(
                                    message_id.clone(),
                                    cx.handle().clone(),
                                    ms_state,
                                ))
                                .map_err(|_| SolidifierError::MissingInxRequester)?;
                            return Ok(());
                        }
                    }
                }
            }

            // If we finished all the parents, that means we have a complete milestone
            // so we should mark it synced
            self.db
                .upsert_sync_record(&SyncRecord {
                    milestone_index: ms_state.milestone_index,
                    logged: false,
                    synced: true,
                })
                .await?;

            cx.addr::<Syncer>()
                .await
                .send(ms_state.milestone_index)
                .map_err(|_| SolidifierError::MissingSyncer)?;

            Ok(())
        }
    }
}
