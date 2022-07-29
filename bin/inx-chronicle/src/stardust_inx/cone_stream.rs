// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use bee_inx::client::Inx;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent},
    types::tangle::MilestoneIndex,
};

use super::InxError;

#[derive(Debug)]
pub struct ConeStream {
    pub milestone_index: MilestoneIndex,
    inx: Inx,
    db: MongoDb,
}

impl ConeStream {
    pub fn new(milestone_index: MilestoneIndex, inx: Inx, db: MongoDb) -> Self {
        Self {
            milestone_index,
            inx,
            db,
        }
    }
}

#[async_trait]
impl Actor for ConeStream {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let cone_stream = self.inx.read_milestone_cone(self.milestone_index.0.into()).await?;
        cx.add_stream(cone_stream);
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
            self.db.complete_milestone(self.milestone_index).await?;
            self.db.set_sync_status_blocks(self.milestone_index).await?;
            self.db.update_ledger_index(self.milestone_index).await?;
            log::debug!("Milestone `{}` synced.", self.milestone_index);
        } else {
            log::warn!("Syncing milestone `{}` failed.", self.milestone_index);
        }
        run_result
    }
}

#[async_trait]
impl HandleEvent<Result<bee_inx::BlockWithMetadata, bee_inx::Error>> for ConeStream {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        block_metadata_result: Result<bee_inx::BlockWithMetadata, bee_inx::Error>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received Stardust block event");
        let bee_inx::BlockWithMetadata { block, metadata } = block_metadata_result?;
        log::trace!("Block id: {:?}", metadata.block_id);

        self.db
            .insert_block_with_metadata(block.clone().inner()?.into(), block.data(), metadata.into())
            .await?;

        log::trace!("Inserted block into database.");

        Ok(())
    }
}
