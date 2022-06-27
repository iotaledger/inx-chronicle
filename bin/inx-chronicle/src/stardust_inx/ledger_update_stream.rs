// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeInclusive;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, HandleEvent, Report},
    types::tangle::MilestoneIndex,
};
use inx::{
    client::InxClient,
    tonic::{Channel, Status},
};

use super::{cone_stream::ConeStream, InxError};

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
impl HandleEvent<Report<ConeStream>> for LedgerUpdateStream {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<ConeStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match event {
            Report::Success(_) => (),
            Report::Error(report) => match report.error {
                ActorError::Result(e) => {
                    Err(e)?;
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.abort().await;
                }
            },
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<Result<inx::proto::LedgerUpdate, Status>> for LedgerUpdateStream {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        ledger_update_result: Result<inx::proto::LedgerUpdate, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received ledger update event {:#?}", ledger_update_result);

        let ledger_update = inx::LedgerUpdate::try_from(ledger_update_result?)?;

        let output_updates_iter = Vec::from(ledger_update.created)
            .into_iter()
            .map(Into::into)
            .chain(Vec::from(ledger_update.consumed).into_iter().map(Into::into));

        self.db.insert_ledger_updates(output_updates_iter).await?;

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

        self.db
            .insert_milestone(milestone_id, milestone_index, milestone_timestamp, payload)
            .await?;

        cx.spawn_child(ConeStream::new(milestone_index, self.inx.clone(), self.db.clone()))
            .await;

        Ok(())
    }
}
