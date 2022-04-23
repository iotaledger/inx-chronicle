// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    bson::{BsonExt, DocError, DocExt},
    db::MongoDatabase,
    runtime::{
        actor::{addr::Addr, context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use mongodb::bson::{doc, document::ValueAccessError};
use thiserror::Error;

use crate::archiver::Archiver;

#[derive(Debug, Error)]
pub enum SolidifierError {
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
pub struct Solidifier {
    pub id: usize,
    db: MongoDatabase,
    archiver_addr: Addr<Archiver>,
}

impl Solidifier {
    pub fn new(id: usize, db: MongoDatabase, archiver_addr: Addr<Archiver>) -> Self {
        Self { id, db, archiver_addr }
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
    use bee_message_stardust::milestone::MilestoneIndex;
    use chronicle::db::model::stardust::message::MessageRecord;

    use super::*;
    use crate::collector::MilestoneState;

    #[cfg(feature = "stardust")]
    #[async_trait]
    impl HandleEvent<MilestoneState> for Solidifier {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            mut state: MilestoneState,
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            // Process by iterating the queue until we either complete the milestone or fail to find a message
            while let Some(message_id) = state.process_queue.front() {
                match state.parents.remove(message_id).flatten() {
                    // The collector received this message
                    Some(parents) => {
                        // Done with this one
                        state.process_queue.pop_front();
                        // Add the parents to be processed
                        state.process_queue.extend(parents);
                    }
                    // The collector never received this message
                    None => {
                        // Try the database first
                        match self
                            .db
                            .doc_collection::<MessageRecord>()
                            .find_one(doc! {"message_id": message_id}, None)
                            .await?
                        {
                            Some(mut message_doc) => {
                                match message_doc.remove("milestone_index").map(|b| b.as_u32()).transpose()? {
                                    Some(ms_index) => {
                                        // We may have reached a different milestone, in which case there is nothing to
                                        // do for this message
                                        if state.milestone_index == ms_index {
                                            let parents = message_doc
                                                .take_array("message.parents")?
                                                .iter()
                                                .map(|b| b.as_string())
                                                .collect::<Result<Vec<String>, _>>()?;
                                            state.process_queue.extend(parents);
                                        }
                                        state.process_queue.pop_front();
                                    }
                                    // If the message has not been referenced, we can't proceed
                                    None => {
                                        // Gotta handle this somehow. Maybe retry later?
                                        todo!()
                                    }
                                }
                            }
                            // Otherwise, send a message to the requester
                            None => {
                                // Send request with a oneshot channel<bool>
                                // Await the channel
                                // If the channel returns true, delay the event
                                // otherwise, we can't complete the milestone, so clear out the data
                                // and throw an error or something
                                todo!("Send a message to the requester")
                            }
                        }
                    }
                }
            }
            // If we finished all the parents, that means we have a complete milestone
            // so we should send it to the archiver now
            self.archiver_addr
                .send(MilestoneIndex(state.milestone_index))
                .map_err(RuntimeError::SendError)?;
            Ok(())
        }
    }
}
