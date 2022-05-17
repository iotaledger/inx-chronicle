// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Instant;

use async_trait::async_trait;
use chronicle::{
    db::{MongoDb, model::stardust::message::{MessageMetadata, MessageRecord}},
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    }, dto,
};
use inx::{client::InxClient, tonic::Channel};
use mongodb::bson::document::ValueAccessError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolidifierError {
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
    inx: InxClient<Channel>,
}

impl Solidifier {
    pub fn new(id: usize, db: MongoDb, inx: InxClient<Channel>) -> Self {
        Self { id, db, inx }
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
    use crate::inx::{collector::stardust::MilestoneState, syncer::{Syncer, NewSyncedMilestone}};

    #[async_trait]
    impl HandleEvent<MilestoneState> for Solidifier {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            mut ms_state: MilestoneState,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            // // Process by iterating the queue until we either complete the milestone or fail to find a message
            // while let Some(message_id) = ms_state.process_queue.front() {
            //     // First check if we already processed this message in this run
            //     if ms_state.visited.contains(message_id) {
            //         ms_state.process_queue.pop_front();
            //     } else {
            //         // Try the database
            //         match self.db.get_message(message_id).await? {
            //             Some(message_rec) => {
            //                 match message_rec
            //                     .metadata
            //                     .map(|metadata| metadata.referenced_by_milestone_index)
            //                 {
            //                     Some(ms_index) => {
            //                         log::trace!(
            //                             "Message {} is referenced by milestone {}",
            //                             message_id.to_hex(),
            //                             ms_index
            //                         );

            //                         // We add the current message to the list of visited messages.
            //                         ms_state.visited.insert(message_id.clone());

            //                         // We may have reached a different milestone, in which case there is nothing to
            //                         // do for this message
            //                         if ms_state.milestone_index == ms_index {
            //                             let parents = Vec::from(message_rec.message.parents);
            //                             ms_state.process_queue.extend(parents);
            //                         }
            //                         ms_state.process_queue.pop_front();
            //                     }
            //                     // If the message has not been referenced, we can't proceed
            //                     None => {
            //                         log::trace!("Requesting metadata for message {}", message_id.to_hex());
            //                         // Send the state and everything. If the requester finds the message, it will
            // circle                         // back.
            //                         cx.addr::<InxWorker>()
            //                             .await
            //                             .send(InxRequest::metadata(message_id.clone(), cx.handle().clone(),
            // ms_state))                             .map_err(|_| SolidifierError::MissingInxRequester)?;
            //                         return Ok(());
            //                     }
            //                 }
            //             }
            //             // Otherwise, send a message to the requester
            //             None => {
            //                 log::trace!("Requesting message {}", message_id.to_hex());
            //                 // Send the state and everything. If the requester finds the message, it will circle
            //                 // back.
            //                 cx.addr::<InxWorker>()
            //                     .await
            //                     .send(InxRequest::message(message_id.clone(), cx.handle().clone(), ms_state))
            //                     .map_err(|_| SolidifierError::MissingInxRequester)?;
            //                 return Ok(());
            //             }
            //         }
            //     }
            // }

            let mut num_iterations = 0usize;
            'parent: while let Some(current_message_id) = ms_state.process_queue.pop_front() {
                if ms_state.visited.contains(&current_message_id) {
                    continue 'parent;
                }

                num_iterations += 1;

                match self.db.get_message(&current_message_id).await? {
                    Some(msg) => {
                        if let Some(md) = msg.metadata {
                            ms_state.visited.insert(current_message_id);

                            let referenced_index = md.referenced_by_milestone_index;
                            if referenced_index != ms_state.milestone_index {
                                continue 'parent;
                            }

                            let parents = msg.message.parents.to_vec();
                            ms_state.process_queue.extend(parents);

                        } else {
                            ms_state.process_queue.push_back(current_message_id.clone());

                            if let Some(metadata) = read_metadata(&mut self.inx, current_message_id.clone()).await {
                                self.db.update_message_metadata(&current_message_id, &metadata).await?;
                            }

                        }
                    }
                    None => {
                        ms_state.process_queue.push_back(current_message_id.clone());

                        if let Some(message) = read_message(&mut self.inx, current_message_id.clone()).await {
                            self.db.upsert_message_record(&message).await?;
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

            println!("Solidification iterations: {}", num_iterations);
            log::debug!(
                "Milestone '{}' synced in {}s.",
                ms_state.milestone_index,
                ms_state.time.elapsed().as_secs_f32()
            );

            // Inform the Syncer about the newly solidified milestone so that it can make progress in case it was a
            // historic one.
            // TODO: this doesn't work
            // if let Some(syncer_addr) = ms_state.syncer_addr.as_ref() {
            //     println!("Syncer notified");
            //     syncer_addr.send(NewSyncedMilestone(ms_state.milestone_index))?;
            // } else {
            //     println!("Syncer NOT notified");
            // }

            cx.addr::<Syncer>()
                .await
                .send(NewSyncedMilestone(ms_state.milestone_index))?;

            Ok(())
        }
    }
}


async fn read_message(inx: &mut InxClient<Channel>, message_id: dto::MessageId) -> Option<MessageRecord> {
    if let (Ok(message), Ok(metadata)) = (
        inx
            .read_message(inx::proto::MessageId {
                id: message_id.0.clone().into(),
            })
            .await,
        inx
            .read_message_metadata(inx::proto::MessageId {
                id: message_id.0.into(),
            })
            .await,
    ) {

        let now = Instant::now();
        let raw = message.into_inner();
        let metadata = metadata.into_inner();
        let message = MessageRecord::try_from((raw, metadata)).unwrap();
        log::warn!("Created MessageRecord. Took {}s.", now.elapsed().as_secs_f32());

        Some(message)
    } else {
        None
    }
}

async fn read_metadata(inx: &mut InxClient<Channel>, message_id: dto::MessageId) -> Option<MessageMetadata>{
    if let Ok(metadata) = inx
        .read_message_metadata(inx::proto::MessageId {
            id: message_id.0.into(),
        })
        .await
    {
        let now = Instant::now();
        let metadata: inx::MessageMetadata = metadata.into_inner().try_into().unwrap();
        let metadata = metadata.into();
        log::warn!("Created metadata. Took {}s.", now.elapsed().as_secs_f32());

        Some(metadata)
    } else {
        None
    }
}
