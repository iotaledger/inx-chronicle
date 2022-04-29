// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{
        bson::{BsonExt, DocError, DocExt},
        MongoDb,
    },
    runtime::{
        actor::{context::ActorContext, event::HandleEvent, Actor},
        error::RuntimeError,
    },
};
use mongodb::bson::document::ValueAccessError;
use thiserror::Error;

use crate::archiver::Archiver;

#[derive(Debug, Error)]
pub enum SolidifierError {
    #[error(transparent)]
    Doc(#[from] DocError),
    #[error("the archiver is missing")]
    MissingArchiver,
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
    use std::str::FromStr;

    use chronicle::{db::model::sync::SyncRecord, stardust::MessageId};

    use super::*;
    use crate::{
        collector::stardust::MilestoneState,
        inx::{InxRequest, InxWorker},
        ADDRESS_REGISTRY,
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
                // Try the database first
                match self.db.get_message(message_id).await? {
                    Some(mut message_doc) => {
                        match message_doc
                            .take_bson("metadata.referenced_by_milestone_index")
                            .ok()
                            .map(|b| b.as_u32())
                            .transpose()?
                        {
                            Some(ms_index) => {
                                // We may have reached a different milestone, in which case there is nothing to
                                // do for this message
                                if ms_state.milestone_index == ms_index {
                                    let parents = message_doc
                                        .take_array("message.parents.inner")?
                                        .iter()
                                        .map(|b| MessageId::from_str(b.as_str().unwrap()))
                                        .collect::<Result<Vec<MessageId>, _>>()
                                        .unwrap();
                                    ms_state.messages.insert(*message_id, message_doc.take_bytes("raw")?);
                                    ms_state.process_queue.extend(parents);
                                }
                                ms_state.process_queue.pop_front();
                            }
                            // If the message has not been referenced, we can't proceed
                            None => {
                                // Send the state and everything. If the requester finds the message, it will circle
                                // back.
                                ADDRESS_REGISTRY
                                    .get::<InxWorker>()
                                    .await
                                    .send(InxRequest::get_metadata(*message_id, cx.handle().clone(), ms_state))
                                    .map_err(|_| SolidifierError::MissingInxRequester)?;
                                return Ok(());
                            }
                        }
                    }
                    // Otherwise, send a message to the requester
                    None => {
                        // Send the state and everything. If the requester finds the message, it will circle
                        // back.
                        ADDRESS_REGISTRY
                            .get::<InxWorker>()
                            .await
                            .send(InxRequest::get_message(*message_id, cx.handle().clone(), ms_state))
                            .map_err(|_| SolidifierError::MissingInxRequester)?;
                        return Ok(());
                    }
                }
            }
            // If we finished all the parents, that means we have a complete milestone
            // so we should mark it synced and send it to the archiver
            self.db
                .upsert_sync_record(&SyncRecord {
                    milestone_index: ms_state.milestone_index,
                    logged: false,
                    synced: true,
                })
                .await?;
            ADDRESS_REGISTRY
                .get::<Archiver>()
                .await
                .send((
                    ms_state.milestone_index,
                    ms_state.messages.into_values().collect::<Vec<_>>(),
                ))
                .map_err(|_| SolidifierError::MissingArchiver)?;
            Ok(())
        }
    }
}
