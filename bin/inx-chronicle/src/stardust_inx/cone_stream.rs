// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent},
    types::tangle::MilestoneIndex,
};
use inx::{tonic::Status, BlockWithMetadata};

use super::InxError;

#[derive(Debug)]
pub struct ConeStream {
    pub milestone_index: MilestoneIndex,
    db: MongoDb,
}

impl ConeStream {
    pub fn new(milestone_index: MilestoneIndex, db: MongoDb) -> Self {
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

    async fn shutdown(
        &mut self,
        _cx: &mut ActorContext<Self>,
        _state: &mut Self::State,
        run_result: Result<(), Self::Error>,
    ) -> Result<(), Self::Error> {
        if run_result.is_ok() {
            self.db.set_sync_status_blocks(self.milestone_index).await?;
            log::debug!("Milestone `{}` synced.", self.milestone_index);
        } else {
            log::warn!("Syncing milestone `{}` failed.", self.milestone_index);
        }
        run_result
    }
}

#[async_trait]
impl HandleEvent<Result<inx::proto::BlockWithMetadata, Status>> for ConeStream {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        block_metadata_result: Result<inx::proto::BlockWithMetadata, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received Stardust Block Event");

        let block_metadata = block_metadata_result?;
        // TODO: Get rid of unwrap here!
        // TODO: Get rid of clone here!
        let raw = block_metadata.block.as_ref().unwrap().data.clone();

        match inx::BlockWithMetadata::try_from(block_metadata) {
            Ok(BlockWithMetadata { block, metadata }) => {
                self.db
                    .insert_block_with_metadata(metadata.block_id.into(), block.into(), raw, metadata.into())
                    .await?;
                log::trace!("Inserted block into database.")
            }
            Err(e) => {
                log::error!("Could not read block: {:?}", e);
            }
        };

        Ok(())
    }
}
