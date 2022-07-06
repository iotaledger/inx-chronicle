// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeInclusive;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent},
    types::{
        ledger::BlockMetadata,
        stardust::block::{Block, BlockId},
        tangle::MilestoneIndex,
    },
};
use futures::StreamExt;
use inx::{
    client::InxClient,
    tonic::{Channel, Status},
    BlockWithMetadata,
};

use super::InxError;

#[derive(Debug)]
pub struct LedgerUpdateStream {
    db: MongoDb,
    inx: InxClient<Channel>,
    range: RangeInclusive<MilestoneIndex>,
}

impl LedgerUpdateStream {
    pub fn new(db: MongoDb, inx: InxClient<Channel>, range: RangeInclusive<MilestoneIndex>) -> Self {
        Self { db, inx, range }
    }
}

#[async_trait]
impl Actor for LedgerUpdateStream {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        let ledger_update_stream = self
            .inx
            .listen_to_ledger_updates(if *self.range.end() == u32::MAX {
                inx::proto::MilestoneRangeRequest::from(*self.range.start()..)
            } else {
                inx::proto::MilestoneRangeRequest::from(self.range.clone())
            })
            .await?
            .into_inner();
        cx.add_stream(ledger_update_stream);
        Ok(())
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        if *self.range.end() == u32::MAX {
            format!("LedgerUpdateStream ({}..)", self.range.start()).into()
        } else {
            format!("LedgerUpdateStream ({}..={})", self.range.start(), self.range.end()).into()
        }
    }
}

#[async_trait]
impl HandleEvent<Result<inx::proto::LedgerUpdate, Status>> for LedgerUpdateStream {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        ledger_update_result: Result<inx::proto::LedgerUpdate, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received ledger update event {:#?}", ledger_update_result);

        let ledger_update = inx::LedgerUpdate::try_from(ledger_update_result?)?;

        let output_updates_iter = Vec::from(ledger_update.created)
            .into_iter()
            .map(Into::into)
            .chain(Vec::from(ledger_update.consumed).into_iter().map(Into::into));

        let mut session = self.db.create_session().await?;
        session.start_transaction(None).await?;

        self.db
            .insert_ledger_updates_with_session(&mut session, output_updates_iter)
            .await?;

        let milestone_request = inx::proto::MilestoneRequest::from_index(ledger_update.milestone_index);

        let milestone_proto = self.inx.read_milestone(milestone_request.clone()).await?.into_inner();

        log::trace!("Received milestone: `{:?}`", milestone_proto);

        let milestone: inx::Milestone = milestone_proto.try_into()?;

        let milestone_index = milestone.milestone_info.milestone_index.into();
        let milestone_timestamp = milestone.milestone_info.milestone_timestamp.into();
        let milestone_id = milestone
            .milestone_info
            .milestone_id
            .ok_or(Self::Error::MissingMilestoneInfo(milestone_index))?
            .into();
        let payload = (&milestone
            .milestone
            .ok_or(Self::Error::MissingMilestoneInfo(milestone_index))?)
            .into();

        let mut cone_stream = self
            .inx
            .read_milestone_cone(inx::proto::MilestoneRequest::from_index(milestone_index.0))
            .await?
            .into_inner()
            .enumerate(); // We enumerate the stream to retreive the white flag index.

        self.db
            .insert_milestone(
                &mut session,
                milestone_id,
                milestone_index,
                milestone_timestamp,
                payload,
            )
            .await?;

        let mut blocks: Vec<(BlockId, Block, Vec<u8>, BlockMetadata, u32)> = Vec::new();
        while let Some((white_flag_index, block_metadata_result)) = cone_stream.next().await {
            log::trace!("Received Stardust block event");
            let block_metadata = block_metadata_result?;
            log::trace!("Block data: {:?}", block_metadata);
            let inx_block_with_metadata: inx::BlockWithMetadata = block_metadata.try_into()?;
            let BlockWithMetadata { metadata, block, raw } = inx_block_with_metadata;

            blocks.push((
                metadata.block_id.into(),
                block.into(),
                raw,
                metadata.into(),
                white_flag_index as u32,
            ))
        }

        log::trace!("Inserting {} blocks into database.", blocks.len());

        if !blocks.is_empty() {
            self.db
                .insert_stream_block_with_metadata_with_session(&mut session, blocks)
                .await?;
        } else {
            log::debug!("Received empty milestone cone.");
        }

        session.commit_transaction().await?;

        self.db.set_sync_status_blocks(milestone_index).await?;
        log::debug!("Milestone `{}` synced.", milestone_index);

        Ok(())
    }
}
