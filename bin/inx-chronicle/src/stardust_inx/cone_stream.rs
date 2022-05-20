// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::{model::stardust::block::BlockRecord, MongoDb},
    runtime::{Actor, ActorContext, HandleEvent},
};
use inx::tonic::Status;

use super::InxError;

#[derive(Debug)]
pub struct ConeStream {
    pub milestone_index: u32,
    db: MongoDb,
}

impl ConeStream {
    pub fn new(milestone_index: u32, db: MongoDb) -> Self {
        Self { milestone_index, db }
    }
}

#[async_trait]
impl Actor for ConeStream {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        format!("ConeStream for milestone {}", self.milestone_index).into()
    }
}

#[async_trait]
impl HandleEvent<Result<inx::proto::BlockWithMetadata, Status>> for ConeStream {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        event: Result<inx::proto::BlockWithMetadata, Status>,
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
