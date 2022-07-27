// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeInclusive;

use async_trait::async_trait;
use bee_inx::client::Inx;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent},
    types::{
        ledger::BlockMetadata,
        stardust::block::Block,
        tangle::{MilestoneIndex, ProtocolInfo, ProtocolParameters},
    },
};
use futures::StreamExt;
use metrics::histogram;
use tokio::time::Instant;

use super::InxError;
use crate::metrics::SYNC_TIME;

#[derive(Debug)]
pub struct LedgerUpdateStream {
    db: MongoDb,
    inx: Inx,
    range: RangeInclusive<MilestoneIndex>,
}

impl LedgerUpdateStream {
    pub fn new(db: MongoDb, inx: Inx, range: RangeInclusive<MilestoneIndex>) -> Self {
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
                (self.range.start().0..).into()
            } else {
                (self.range.start().0..=self.range.end().0).into()
            })
            .await?;
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
impl HandleEvent<Result<bee_inx::LedgerUpdate, bee_inx::Error>> for LedgerUpdateStream {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        ledger_update_result: Result<bee_inx::LedgerUpdate, bee_inx::Error>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received ledger update event {:#?}", ledger_update_result);

        let ledger_update_start = Instant::now();
        let ledger_update = ledger_update_result?;

        let output_updates = Vec::from(ledger_update.created)
            .into_iter()
            .map(TryInto::try_into)
            .chain(Vec::from(ledger_update.consumed).into_iter().map(TryInto::try_into))
            .collect::<Result<Vec<_>, _>>()?;

        self.db.insert_ledger_updates(output_updates.into_iter()).await?;

        let milestone = self.inx.read_milestone(ledger_update.milestone_index.into()).await?;
        let parameters: ProtocolParameters = self
            .inx
            .read_protocol_parameters(ledger_update.milestone_index.into())
            .await?
            .inner()?
            .into();

        self.db
            .set_protocol_parameters(ProtocolInfo {
                parameters,
                tangle_index: ledger_update.milestone_index.into(),
            })
            .await?;

        log::trace!("Received milestone: `{:?}`", milestone);

        let milestone_index = milestone.milestone_info.milestone_index.into();
        let milestone_timestamp = milestone.milestone_info.milestone_timestamp.into();
        let milestone_id = milestone
            .milestone_info
            .milestone_id
            .ok_or(Self::Error::MissingMilestoneInfo(milestone_index))?
            .into();
        let payload = Into::into(
            &milestone
                .milestone
                .ok_or(Self::Error::MissingMilestoneInfo(milestone_index))?,
        );

        self.db
            .insert_milestone(milestone_id, milestone_index, milestone_timestamp, payload)
            .await?;

        let mut cone_stream = self.inx.read_milestone_cone(milestone_index.0.into()).await?;

        while let Some(bee_inx_block_with_metadata) = cone_stream.next().await.transpose()? {
            log::trace!(
                "Received Stardust block with metadata: {:?}",
                bee_inx_block_with_metadata
            );

            let bee_inx::BlockWithMetadata { block, metadata } = bee_inx_block_with_metadata;
            let raw = block.clone().data();
            let block: Block = block.inner()?.into();
            let metadata: BlockMetadata = metadata.into();

            self.db.insert_block_with_metadata(block, raw, metadata).await?;
            log::trace!("Inserted block into database.");
        }

        self.db.set_sync_status_blocks(milestone_index).await?;
        self.db.update_ledger_index(milestone_index).await?;

        histogram!(SYNC_TIME, ledger_update_start.elapsed());
        log::debug!("Milestone `{}` synced.", milestone_index);

        Ok(())
    }
}
