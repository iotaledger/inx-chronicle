// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeInclusive;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, ConfigureActor, HandleEvent, Report},
    types::{ledger::OutputWithMetadata, tangle::MilestoneIndex},
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

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        if *self.range.end() == u32::MAX {
            format!("Milestone Stream ({}..)", self.range.start()).into()
        } else {
            format!("Milestone Stream ({}..={})", self.range.start(), self.range.end()).into()
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

        let output_updates_iter = ledger_update
            .created
            .iter()
            .cloned()
            .map(OutputWithMetadata::from)
            .chain(ledger_update.consumed.iter().cloned().map(OutputWithMetadata::from));

        self.db.insert_ledger_updates(output_updates_iter).await?;

        let milestone_request = inx::proto::MilestoneRequest::from_index(ledger_update.milestone_index);

        let milestone: inx::Milestone = self
            .inx
            .read_milestone(milestone_request.clone())
            .await?
            .into_inner()
            .try_into()?;

        let milestone_index = milestone.milestone_info.milestone_index.into();
        let milestone_timestamp = milestone.milestone_info.milestone_timestamp.into();
        let milestone_id = milestone.milestone_info.milestone_id.into();
        let payload = (&milestone.milestone).into();

        self.db
            .insert_milestone(milestone_id, milestone_index, milestone_timestamp, payload)
            .await?;

        let cone_stream = self.inx.read_milestone_cone(milestone_request).await?.into_inner();

        cx.spawn_child(ConeStream::new(milestone_index, self.db.clone()).with_stream(cone_stream))
            .await;

        Ok(())
    }
}
