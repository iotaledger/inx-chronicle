// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent},
    types::tangle::MilestoneIndex,
};
use inx::{
    client::InxClient,
    tonic::{Channel, Status},
    BlockWithMetadata,
};

use super::InxError;

#[derive(Debug)]
pub struct ConeStream {
    pub milestone_index: MilestoneIndex,
    inx_client: InxClient<Channel>,
    db: MongoDb,
}

impl ConeStream {
    pub fn new(milestone_index: MilestoneIndex, inx_client: InxClient<Channel>, db: MongoDb) -> Self {
        Self {
            milestone_index,
            inx_client,
            db,
        }
    }
}

#[async_trait]
impl Actor for ConeStream {
    type State = u32;
    type Error = InxError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let cone_stream = self
            .inx_client
            .read_milestone_cone(inx::proto::MilestoneRequest::from_index(self.milestone_index.0))
            .await?
            .into_inner();
        cx.add_stream(cone_stream);
        Ok(0)
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
        white_flag_index: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received Stardust block event");
        let block_metadata = block_metadata_result?;
        log::trace!("Block data: {:?}", block_metadata);
        let inx_block_with_metadata: inx::BlockWithMetadata = block_metadata.try_into()?;
        let BlockWithMetadata { metadata, block, raw } = inx_block_with_metadata;

        self.db
            .insert_block_with_metadata(
                metadata.block_id.into(),
                block.into(),
                raw,
                metadata.into(),
                *white_flag_index,
            )
            .await?;
        *white_flag_index += 1;

        log::trace!("Inserted block into database.");

        Ok(())
    }
}
