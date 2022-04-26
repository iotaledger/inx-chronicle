// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    bson::{BsonExt, DocError, DocExt},
    db::MongoDatabase,
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
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
}

impl Solidifier {
    pub fn new(id: usize, db: MongoDatabase) -> Self {
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
    use std::str::FromStr;

    use chronicle::{
        db::model::stardust::message::MessageRecord,
        stardust::{milestone::MilestoneIndex, MessageId},
    };
    use packable::PackableExt;

    use super::*;
    use crate::{collector::stardust::MilestoneState, inx::InxRequester, ADDRESS_REGISTRY};

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
                match ms_state.messages.get(message_id) {
                    // The collector received this message
                    Some(message) => {
                        ms_state.raw_messages.insert(*message_id, message.pack_to_vec());
                        // Done with this one
                        ms_state.process_queue.pop_front();
                        // Add the parents to be processed
                        ms_state.process_queue.extend(message.parents().iter());
                    }
                    // The collector never received this message
                    None => {
                        // Try the database first
                        match self
                            .db
                            .doc_collection::<MessageRecord>()
                            .find_one(doc! {"message_id": message_id.to_string()}, None)
                            .await?
                        {
                            Some(mut message_doc) => {
                                match message_doc.remove("milestone_index").map(|b| b.as_u32()).transpose()? {
                                    Some(ms_index) => {
                                        // We may have reached a different milestone, in which case there is nothing to
                                        // do for this message
                                        if ms_state.milestone_index == ms_index {
                                            let parents = message_doc
                                                .take_array("message.parents")?
                                                .iter()
                                                .map(|b| MessageId::from_str(b.as_str().unwrap()))
                                                .collect::<Result<Vec<MessageId>, _>>()
                                                .unwrap();
                                            ms_state
                                                .raw_messages
                                                .insert(*message_id, message_doc.take_bytes("message.raw")?);
                                            ms_state.process_queue.extend(parents);
                                        }
                                        ms_state.process_queue.pop_front();
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
                                // Send the state and everything. If the requester finds the message, it will circle
                                // back.
                                ADDRESS_REGISTRY
                                    .get::<InxRequester>()
                                    .await
                                    .send((*message_id, cx.handle().clone(), ms_state))
                                    .map_err(RuntimeError::SendError)?;
                                return Ok(());
                            }
                        }
                    }
                }
            }
            // If we finished all the parents, that means we have a complete milestone
            // so we should send it to the archiver now
            ADDRESS_REGISTRY
                .get::<Archiver>()
                .await
                .send((
                    MilestoneIndex(ms_state.milestone_index),
                    ms_state.raw_messages.into_values().collect::<Vec<_>>(),
                ))
                .map_err(RuntimeError::SendError)?;
            Ok(())
        }
    }
}
