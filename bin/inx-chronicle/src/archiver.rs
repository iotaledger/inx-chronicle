// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{model::sync::SyncRecord, MongoDatabase, MongoDbError},
    runtime::actor::{context::ActorContext, event::HandleEvent, Actor},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiverError {
    #[error(transparent)]
    MongoDb(#[from] MongoDbError),
}

#[derive(Debug)]
pub struct Archiver {
    db: MongoDatabase,
}

impl Archiver {
    pub fn new(db: MongoDatabase) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Actor for Archiver {
    type State = ();
    type Error = ArchiverError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        // TODO
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<(u32, Vec<Vec<u8>>)> for Archiver {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        (milestone_index, _messages): (u32, Vec<Vec<u8>>),
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::info!("Archiving milestone {}", milestone_index);
        // TODO: Actually archive the messages
        self.db
            .upsert_one(&SyncRecord {
                milestone_index,
                logged: true,
                synced: true,
            })
            .await?;
        Ok(())
    }
}
