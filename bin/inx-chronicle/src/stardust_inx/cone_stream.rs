// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{
        model::stardust::block::{BlockRecord, BlockWithMetadata},
        MongoDb,
    },
    runtime::{Actor, ActorContext, HandleEvent},
};
use inx::tonic::Status;

use super::InxError;

#[derive(Debug)]
pub struct ConeStream {
    db: MongoDb,
}

impl ConeStream {
    pub fn new(db: MongoDb) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Actor for ConeStream {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Result<BlockWithMetadata, Status>> for ConeStream {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        event: Result<BlockWithMetadata, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received Stardust Block Event");
        match BlockRecord::try_from(event?) {
            Ok(rec) => {
                self.db.upsert_block_record(&rec).await?;
            }
            Err(e) => {
                log::error!("Could not read block: {:?}", e);
            }
        };
        Ok(())
    }
}
